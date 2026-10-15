//! Stamp frames, push them through a real x264 encode (the thing YouTube does to us), decode them back.

use fire_core::sync::{decode, stamp, SyncCode};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const FRAMES: usize = 30;
const BASE_ID: u32 = 0x00ABCD00;
const BASE_TICK: u32 = 0x00123400;

/// Three rendered frames per tick, like the real game at ~60 fps.
fn expected(i: usize) -> SyncCode {
    SyncCode {
        tick: BASE_TICK + (i / 3) as u32,
        frame_id: BASE_ID + i as u32,
    }
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fire_core_{}_{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Moving gradient + blocky texture so x264 has real work to do (not a flat frame).
fn background(w: usize, h: usize, t: usize) -> Vec<u8> {
    let mut f = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let block = (((x / 16 + y / 16 + t) * 37) % 90) as u32;
            f[y * w + x] = (((x + t * 7) * 255 / w) as u32 / 2 + block) as u8;
        }
    }
    f
}

fn roundtrip(name: &str, w: usize, h: usize, crf: u32, out: Option<(usize, usize)>) {
    if !ffmpeg_available() {
        eprintln!("ffmpeg not found, skipping {name}");
        return;
    }
    let dir = scratch(name);
    let raw_in = dir.join("in.raw");
    let video = dir.join("out.mp4");

    let mut raw = Vec::with_capacity(w * h * FRAMES);
    for i in 0..FRAMES {
        let mut f = background(w, h, i);
        let e = expected(i);
        stamp(&mut f, w, h, e.tick, e.frame_id).unwrap();
        raw.extend_from_slice(&f);
    }
    std::fs::write(&raw_in, &raw).unwrap();

    let ok = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray",
        ])
        .args(["-s", &format!("{w}x{h}"), "-r", "60", "-i"])
        .arg(&raw_in)
        .args([
            "-c:v",
            "libx264",
            "-crf",
            &crf.to_string(),
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&video)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ffmpeg encode failed");

    let (ow, oh) = out.unwrap_or((w, h));
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error", "-i"]).arg(&video);
    if out.is_some() {
        cmd.args(["-vf", &format!("scale={ow}:{oh}")]);
    }
    let decoded = cmd
        .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    assert!(decoded.status.success(), "ffmpeg decode failed");

    let frame_len = ow * oh;
    assert_eq!(
        decoded.stdout.len(),
        frame_len * FRAMES,
        "frame count changed"
    );
    let mut bad = Vec::new();
    for (i, f) in decoded.stdout.chunks_exact(frame_len).enumerate() {
        if decode(f, ow, oh) != Some(expected(i)) {
            bad.push(i);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(bad.is_empty(), "{name}: frames not decoded: {bad:?}");
}

#[test]
fn x264_crf28_1080p() {
    roundtrip("crf28_1080p", 1920, 1080, 28, None);
}

#[test]
fn x264_crf35_1080p_heavy_compression() {
    roundtrip("crf35_1080p", 1920, 1080, 35, None);
}

#[test]
fn x264_then_downscaled_to_720p() {
    roundtrip("down720", 1920, 1080, 30, Some((1280, 720)));
}

#[test]
fn x264_then_downscaled_to_480p() {
    roundtrip("down480", 1920, 1080, 30, Some((854, 480)));
}
