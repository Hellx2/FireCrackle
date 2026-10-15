//! End to end: build a real x264 video with awkward frames, run the `fire-decode` binary on it.

use fire_core::sync::stamp;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_fire-decode");
const W: usize = 1920;
const H: usize = 1080;
const FRAMES: usize = 24;

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn have_ffmpeg() -> bool {
    let ok = tool_available("ffmpeg") && tool_available("ffprobe");
    if !ok {
        eprintln!("ffmpeg/ffprobe not found, skipping");
    }
    ok
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fire_decode_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn background(t: usize) -> Vec<u8> {
    let mut f = vec![0u8; W * H];
    for y in 0..H {
        for x in 0..W {
            let block = (((x / 16 + y / 16 + t) * 37) % 90) as u32;
            f[y * W + x] = (((x + t * 7) * 255 / W) as u32 / 2 + block) as u8;
        }
    }
    f
}

/// What each video frame carries: `None` = no strip at all.
fn spec(i: usize) -> Option<(u32, u32)> {
    match i {
        5 | 6 => None,                      // unstamped
        10 => Some((1000 + 9 / 3, 509)),    // a duplicate of frame 9
        15 => Some((1000 + 15 / 3, 9000)),  // an id that has no label row
        _ => Some((1000 + (i / 3) as u32, 500 + i as u32)),
    }
}

fn make_video(dir: &Path) -> PathBuf {
    let raw = dir.join("in.raw");
    let video = dir.join("out.mp4");
    let mut bytes = Vec::with_capacity(W * H * FRAMES);
    for i in 0..FRAMES {
        let mut f = background(i);
        if let Some((tick, id)) = spec(i) {
            stamp(&mut f, W, H, tick, id).unwrap();
        }
        bytes.extend_from_slice(&f);
    }
    std::fs::write(&raw, &bytes).unwrap();
    let ok = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "gray", "-s"])
        .arg(format!("{W}x{H}"))
        .args(["-r", "60", "-i"])
        .arg(&raw)
        .args(["-c:v", "libx264", "-crf", "28", "-pix_fmt", "yuv420p"])
        .arg(&video)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ffmpeg encode failed");
    video
}

/// Label rows for ids 500..523. The row for id 512 carries a wrong tick.
fn make_labels(dir: &Path) -> PathBuf {
    let path = dir.join("labels.jsonl");
    let mut s = String::new();
    for i in 0..FRAMES {
        let id = 500 + i;
        let mut tick = 1000 + i / 3;
        if id == 512 {
            tick += 1; // wrong on purpose
        }
        s.push_str(&format!("{{\"f\":{id},\"tick\":{tick},\"f0\":3,\"f1\":19,\"fire\":false,\"fires\":[]}}\n"));
    }
    std::fs::write(&path, s).unwrap();
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().unwrap()
}

fn lines(o: &Output) -> Vec<String> {
    String::from_utf8_lossy(&o.stdout).lines().map(|s| s.to_string()).collect()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

// ---------- the main scenario ----------

#[test]
fn decodes_and_joins_every_frame() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("main");
    let video = make_video(&dir);
    let labels = make_labels(&dir);
    let out = run(&[video.to_str().unwrap(), "--labels", labels.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let rows = lines(&out);
    assert_eq!(rows.len(), FRAMES, "one row per video frame, none duplicated or dropped");

    for (n, row) in rows.iter().enumerate() {
        assert!(row.starts_with(&format!("{{\"n\":{n},")), "row {n} out of order: {row}");
    }
    // the unstamped frames are reported as undecodable, not guessed
    for n in [5, 6] {
        assert_eq!(rows[n], format!("{{\"n\":{n},\"tick\":null,\"f\":null,\"tick_ok\":null,\"label\":null}}"));
    }
    // a normal frame: strip decoded, label row attached verbatim, tick agrees
    assert!(
        rows[0].starts_with("{\"n\":0,\"tick\":1000,\"f\":500,\"tick_ok\":true,\"label\":{\"f\":500,\"tick\":1000,"),
        "{}",
        rows[0]
    );
    assert!(rows[0].ends_with("\"fires\":[]}}"), "{}", rows[0]);
    // the duplicated frame decodes to the same id as the frame before it
    assert!(rows[9].contains("\"f\":509"), "{}", rows[9]);
    assert!(rows[10].contains("\"f\":509"), "{}", rows[10]);
    // wrong tick in the label log -> flagged, label still attached
    assert!(rows[12].contains("\"f\":512") && rows[12].contains("\"tick_ok\":false"), "{}", rows[12]);
    // an id the label log does not have
    assert_eq!(rows[15], "{\"n\":15,\"tick\":1005,\"f\":9000,\"tick_ok\":null,\"label\":null}");

    let err = stderr(&out);
    for key in [
        "video=1920x1080", "frames=24", "decoded=22", "undecodable=2", "matched=21", "unmatched=1",
        "tick_mismatch=1", "id_repeats=1", "id_decreases=1",
    ] {
        assert!(err.contains(key), "summary lacks {key}: {err}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_labels_the_rows_are_short() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("nolabels");
    let video = make_video(&dir);
    let out = run(&[video.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let rows = lines(&out);
    assert_eq!(rows.len(), FRAMES);
    assert_eq!(rows[0], "{\"n\":0,\"tick\":1000,\"f\":500}");
    assert_eq!(rows[5], "{\"n\":5,\"tick\":null,\"f\":null}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn out_writes_a_file_and_leaves_stdout_empty() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("outfile");
    let video = make_video(&dir);
    let labels = make_labels(&dir);
    let target = dir.join("joined.jsonl");
    let out = run(&[video.to_str().unwrap(), "--labels", labels.to_str().unwrap(), "--out", target.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(out.stdout.is_empty());
    let text = std::fs::read_to_string(&target).unwrap();
    assert_eq!(text.lines().count(), FRAMES);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------- failures: clear message, non-zero exit, never a panic ----------

#[test]
fn a_missing_video_is_a_clean_error() {
    let out = run(&["definitely_not_a_video_file.mp4"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.to_lowercase().contains("error"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn a_missing_label_file_is_a_clean_error() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("nolabelfile");
    let video = make_video(&dir);
    let out = run(&[video.to_str().unwrap(), "--labels", dir.join("nope.jsonl").to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr(&out).to_lowercase().contains("error"));
    assert!(!stderr(&out).contains("panicked"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_is_not_a_video_is_a_clean_error() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("garbage");
    let p = dir.join("fake.mp4");
    std::fs::write(&p, "this is not a video").unwrap();
    let out = run(&[p.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_video_too_narrow_for_the_strip_is_refused() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("narrow");
    let p = dir.join("small.mp4");
    let ok = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=320x180:rate=10", "-t", "1"])
        .args(["-pix_fmt", "yuv420p"])
        .arg(&p)
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let out = run(&[p.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr(&out).to_lowercase().contains("narrow"), "{}", stderr(&out));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_arguments_print_usage_and_exit_2() {
    let none = run(&[]);
    assert_eq!(none.status.code(), Some(2));
    assert!(stderr(&none).contains("Usage"), "{}", stderr(&none));
    let bogus = run(&["video.mp4", "--bogus"]);
    assert_eq!(bogus.status.code(), Some(2));
    assert!(stderr(&bogus).contains("Usage"));
    let dangling = run(&["video.mp4", "--labels"]);
    assert_eq!(dangling.status.code(), Some(2));
}
