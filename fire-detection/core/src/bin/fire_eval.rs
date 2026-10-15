//! Scores the baseline template matcher: for every labelled fire part, takes its (oracle) box from the joined
//! decode output, matches the video pixels against the 128 fire candidates and compares with the label.
//!
//! Usage: fire-eval VIDEO JOINED.jsonl [--stride N] [--assets DIR]
//! JOINED is the output of fire-decode; row n is video frame n. DIR holds fire_0.rgba and fire_1.rgba.

use fire_core::detect::{classify, Candidate, Templates};
use fire_core::eval::{bucket_dist, bucket_size, bucket_veil, bucket_vis, judge, Stats};
use fire_core::json::{parse, Value};
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut stride = 1u64;
    let mut assets = "data/assets".to_string();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--stride" => {
                stride = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&s| s > 0)
                    .unwrap_or_else(|| die("--stride needs a positive number".into()))
            }
            "--assets" => {
                assets = it
                    .next()
                    .unwrap_or_else(|| die("--assets needs a directory".into()))
            }
            _ if video.is_none() => video = Some(a),
            _ if joined.is_none() => joined = Some(a),
            _ => die(format!("unexpected argument {a}")),
        }
    }
    let (Some(video), Some(joined)) = (video, joined) else {
        die("usage: fire-eval VIDEO JOINED.jsonl [--stride N] [--assets DIR]".into())
    };

    let s0 = std::fs::read(format!("{assets}/fire_0.rgba"))
        .unwrap_or_else(|e| die(format!("fire_0.rgba: {e}")));
    let s1 = std::fs::read(format!("{assets}/fire_1.rgba"))
        .unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));
    let templates = Templates::from_rgba(&s0, &s1).unwrap_or_else(|e| die(e));

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
        .arg(&video)
        .output()
        .unwrap_or_else(|e| die(format!("ffprobe: {e}")));
    let info = parse_probe(&String::from_utf8_lossy(&probe.stdout)).unwrap_or_else(|e| die(e));

    let mut child = Command::new("ffmpeg")
        .args(["-nostdin", "-loglevel", "error", "-nostats", "-i"])
        .arg(&video)
        .args([
            "-fps_mode",
            "passthrough",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "-",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die(format!("ffmpeg: {e}")));
    let mut pipe = child.stdout.take().expect("stdout was piped");
    let mut frame = vec![0u8; info.width * info.height * 3];

    let rows =
        BufReader::new(File::open(&joined).unwrap_or_else(|e| die(format!("{joined}: {e}"))));
    let mut stats = Stats::default();
    let (mut errors, mut frames_used) = (0u64, 0u64);
    let mut n = 0u64;
    for line in rows.lines() {
        let line = line.unwrap_or_else(|e| die(format!("reading {joined}: {e}")));
        match read_frame(&mut pipe, &mut frame) {
            Ok(true) => {}
            Ok(false) => die(format!(
                "video ended at frame {n} but {joined} has more rows"
            )),
            Err(e) => die(format!("reading video: {e}")),
        }
        let this = n;
        n += 1;
        if !this.is_multiple_of(stride) {
            continue;
        }
        let Ok(row) = parse(&line) else { continue };
        let Some(label) = row.get("label") else {
            continue;
        };
        let (Some(lw), Some(fires)) = (
            label.get("w").and_then(Value::num),
            label.get("fires").and_then(Value::arr),
        ) else {
            continue;
        };
        if fires.is_empty() || lw <= 0.0 {
            continue;
        }
        frames_used += 1;
        let scale = info.width as f64 / lw;
        for f in fires {
            let src = f.get("src").and_then(Value::str).unwrap_or("?");
            let dist = f.get("dist").and_then(Value::num).unwrap_or(0.0);
            let Some(parts) = f.get("parts").and_then(Value::arr) else {
                continue;
            };
            for p in parts {
                let (Some(sheet), Some(mirror), Some(prow), Some(bx)) = (
                    p.get("sheet").and_then(Value::num),
                    p.get("mirror").and_then(Value::bool),
                    p.get("row").and_then(Value::num),
                    p.get("box").and_then(Value::arr),
                ) else {
                    continue;
                };
                let b: Vec<f64> = bx.iter().filter_map(Value::num).collect();
                if b.len() != 4 {
                    continue;
                }
                let bbox = [b[0] * scale, b[1] * scale, b[2] * scale, b[3] * scale];
                let truth = Candidate {
                    sheet: sheet as u8,
                    mirror,
                    row: prow as u8,
                };
                let ranked = match classify(&templates, &frame, info.width, info.height, bbox) {
                    Ok(r) => r,
                    Err(_) => {
                        errors += 1;
                        continue;
                    }
                };
                let vis = p.get("vis").and_then(Value::num).unwrap_or(0.0);
                let veil = p.get("veil").and_then(Value::num).unwrap_or(0.0);
                let size = (bbox[2] - bbox[0]).max(bbox[3] - bbox[1]);
                let src_key = format!("src {src}");
                let grp_key = format!(
                    "truth sheet{} {}",
                    truth.sheet,
                    if truth.mirror { "mirrored" } else { "normal" }
                );
                stats.record(
                    &[
                        &src_key,
                        &grp_key,
                        bucket_vis(vis),
                        bucket_veil(veil),
                        bucket_dist(dist),
                        bucket_size(size),
                    ],
                    judge(&templates, truth, &ranked),
                );
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    println!("fire-eval: frames={n} with_fire_used={frames_used} classify_errors={errors} stride={stride}");
    print!("{}", stats.table());
}
