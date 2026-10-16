//! `fire-decode VIDEO [--labels labels.jsonl] [--out joined.jsonl]`
//!
//! Streams a recording through ffmpeg, reads the sync strip of every frame and writes one JSONL row per video
//! frame: the strip's tick and frame id and, with `--labels`, the matching row of the mod's label log.
//! Format and failure modes: `sync/PLAN.md`.

use fire_core::labels::LabelIndex;
use fire_core::report::{format_row, LabelResult, Summary};
use fire_core::sync::{decode, strip_size};
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{self, BufWriter, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const USAGE: &str = "Usage: fire-decode VIDEO [--labels labels.jsonl] [--out joined.jsonl]";

struct Failure {
    code: i32,
    message: String,
}

fn fail<T>(code: i32, message: impl Into<String>) -> Result<T, Failure> {
    Err(Failure {
        code,
        message: message.into(),
    })
}

struct Args {
    video: PathBuf,
    labels: Option<PathBuf>,
    out: Option<PathBuf>,
}

fn parse_args(args: Vec<String>) -> Result<Args, Failure> {
    let usage = |why: &str| -> Failure {
        Failure {
            code: 2,
            message: if why.is_empty() {
                USAGE.to_string()
            } else {
                format!("error: {why}\n{USAGE}")
            },
        }
    };
    let mut video = None;
    let mut labels = None;
    let mut out = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--labels" => {
                labels = Some(PathBuf::from(
                    it.next().ok_or_else(|| usage("--labels needs a file"))?,
                ))
            }
            "--out" => {
                out = Some(PathBuf::from(
                    it.next().ok_or_else(|| usage("--out needs a file"))?,
                ))
            }
            s if s.starts_with("--") => return Err(usage(&format!("unknown option {s}"))),
            _ if video.is_none() => video = Some(PathBuf::from(a)),
            _ => return Err(usage("more than one video given")),
        }
    }
    match video {
        Some(video) => Ok(Args { video, labels, out }),
        None => Err(usage("")),
    }
}

fn run() -> Result<(), Failure> {
    let args = parse_args(std::env::args().skip(1).collect())?;
    if !args.video.is_file() {
        return fail(
            1,
            format!("error: video not found: {}", args.video.display()),
        );
    }

    // size of the video
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
        ])
        .args(["-of", "default=noprint_wrappers=1"])
        .arg(&args.video)
        .output()
        .map_err(|e| Failure {
            code: 1,
            message: format!(
                "error: could not run ffprobe (is ffmpeg installed and on PATH?): {e}"
            ),
        })?;
    if !probe.status.success() {
        return fail(
            1,
            format!(
                "error: ffprobe could not read the video: {}",
                String::from_utf8_lossy(&probe.stderr).trim()
            ),
        );
    }
    let info = parse_probe(&String::from_utf8_lossy(&probe.stdout)).map_err(|e| Failure {
        code: 1,
        message: format!("error: {e}"),
    })?;
    let Some((_, strip_h)) = strip_size(info.width) else {
        return fail(
            1,
            format!(
                "error: video is too narrow ({} px): the sync strip needs at least 480 px of width",
                info.width
            ),
        );
    };
    // the decoder only needs the strip rows; keep the crop height even for the chroma subsampling
    let crop_h = strip_h + strip_h % 2;
    if info.height < crop_h {
        return fail(
            1,
            format!(
                "error: video is too short ({} px) for the sync strip",
                info.height
            ),
        );
    }

    let mut index = match &args.labels {
        Some(p) => Some(LabelIndex::build(p).map_err(|e| Failure {
            code: 1,
            message: format!("error: cannot read labels {}: {e}", p.display()),
        })?),
        None => None,
    };

    let mut out: Box<dyn Write> = match &args.out {
        Some(p) => Box::new(BufWriter::new(File::create(p).map_err(|e| Failure {
            code: 1,
            message: format!("error: cannot create {}: {e}", p.display()),
        })?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    };

    // -fps_mode passthrough: one output frame per decoded frame, no duplicates or drops, so n is the real frame index
    let mut child = Command::new("ffmpeg")
        .args(["-nostdin", "-loglevel", "error", "-nostats", "-i"])
        .arg(&args.video)
        .args([
            fire_core::video::PASSTHROUGH_ARGS[0],
            fire_core::video::PASSTHROUGH_ARGS[1],
            "-vf",
        ])
        .arg(format!("crop=iw:{crop_h}:0:0"))
        .args(["-pix_fmt", "gray", "-f", "rawvideo", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| Failure {
            code: 1,
            message: format!("error: could not run ffmpeg (is it installed and on PATH?): {e}"),
        })?;
    let mut pipe = child.stdout.take().expect("stdout was piped");

    let mut buf = vec![0u8; info.width * crop_h];
    let mut summary = Summary::default();
    let mut n: u64 = 0;
    loop {
        match read_frame(&mut pipe, &mut buf) {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => {
                let _ = child.kill();
                let _ = out.flush();
                return fail(
                    3,
                    format!("error: video stream ended unexpectedly after {n} frames: {e}"),
                );
            }
        }
        let code = decode(&buf, info.width, crop_h);
        let label = match (&mut index, code) {
            (None, _) => LabelResult::NotRequested,
            (Some(_), None) => LabelResult::Missing,
            (Some(idx), Some(c)) => match idx.get(c.frame_id) {
                Ok(Some((tick, line))) => LabelResult::Found { tick, line },
                Ok(None) => LabelResult::Missing,
                Err(e) => {
                    let _ = child.kill();
                    return fail(1, format!("error: cannot read the label log: {e}"));
                }
            },
        };
        summary.record(code, &label);
        if let Err(e) = writeln!(out, "{}", format_row(n, code, &label)) {
            let _ = child.kill();
            if e.kind() == ErrorKind::BrokenPipe {
                return Ok(()); // e.g. `| head`: the reader went away, that is fine
            }
            return fail(1, format!("error: cannot write the output: {e}"));
        }
        n += 1;
    }
    if let Err(e) = out.flush() {
        if e.kind() != ErrorKind::BrokenPipe {
            return fail(1, format!("error: cannot write the output: {e}"));
        }
    }
    drop(pipe);
    let status = child.wait().map_err(|e| Failure {
        code: 3,
        message: format!("error: waiting for ffmpeg: {e}"),
    })?;
    if !status.success() {
        return fail(
            3,
            format!("error: ffmpeg failed after {n} frames ({status})"),
        );
    }

    eprintln!(
        "fire-decode: video={}x{} {}",
        info.width,
        info.height,
        summary.to_line()
    );
    if let Some(idx) = &index {
        eprintln!(
            "fire-decode: labels rows={} bad_lines={} duplicate_ids={}",
            idx.rows, idx.bad_lines, idx.duplicate_ids
        );
    }
    Ok(())
}

fn main() {
    if let Err(f) = run() {
        eprintln!("{}", f.message);
        std::process::exit(f.code);
    }
}
