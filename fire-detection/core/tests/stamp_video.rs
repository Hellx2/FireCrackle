//! `stamp-test-video` makes a noisy synthetic video whose frame i carries tick 1000 + i/3 and id i in its strip.
//! It exists so the compression robustness of the strip can be measured with real encoders.

use std::path::PathBuf;
use std::process::{Command, Stdio};

const GEN: &str = env!("CARGO_BIN_EXE_stamp-test-video");
const DECODE: &str = env!("CARGO_BIN_EXE_fire-decode");

fn have_ffmpeg() -> bool {
    ["ffmpeg", "ffprobe"].iter().all(|t| {
        Command::new(t)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stamp_video_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_generated_video_decodes_back_to_its_ids_and_ticks() {
    if !have_ffmpeg() {
        eprintln!("ffmpeg not found, skipping");
        return;
    }
    let dir = scratch("ok");
    let video = dir.join("v.mp4");
    let gen = Command::new(GEN)
        .arg(&video)
        .args(["--frames", "12", "--size", "1280x720", "--fps", "30"])
        .output()
        .unwrap();
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    assert!(video.is_file());

    let out = Command::new(DECODE).arg(&video).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 12);
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(
            *row,
            format!("{{\"n\":{i},\"tick\":{},\"f\":{i}}}", 1000 + i / 3),
            "row {i}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_arguments_print_usage_and_exit_2() {
    for args in [
        vec![],
        vec!["out.mp4", "--size", "abc"],
        vec!["out.mp4", "--size", "100x100"], // too narrow for the strip
        vec!["out.mp4", "--frames", "0"],
        vec!["out.mp4", "--fps", "0"],
        vec!["out.mp4", "--bogus"],
        vec!["out.mp4", "--frames"],
    ] {
        let o = Command::new(GEN).args(&args).output().unwrap();
        assert_eq!(
            o.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("Usage"),
            "{args:?}"
        );
    }
}

#[test]
fn an_unwritable_output_is_a_clean_error() {
    if !have_ffmpeg() {
        return;
    }
    let dir = scratch("nowrite");
    let o = Command::new(GEN)
        .arg(dir.join("no_such_dir").join("v.mp4"))
        .args(["--frames", "2", "--size", "640x360"])
        .output()
        .unwrap();
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.to_lowercase().contains("error") && !err.contains("panicked"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
