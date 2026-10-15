//! `stamp-test-video OUT.mp4 [--frames N] [--size WxH] [--fps F]`
//!
//! Writes a high-quality but deliberately noisy synthetic video (moving gradient, blocks, grain, hard-edged
//! rectangles) whose frame `i` carries tick `1000 + i/3` and frame id `i` in the sync strip. It is the source
//! for measuring how well the strip survives real encoders (augment/robustness.py).

use fire_core::sync::{stamp, strip_size};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const USAGE: &str = "Usage: stamp-test-video OUT.mp4 [--frames N] [--size WxH] [--fps F]";

fn usage(why: &str) -> ! {
    if why.is_empty() {
        eprintln!("{USAGE}");
    } else {
        eprintln!("error: {why}\n{USAGE}");
    }
    std::process::exit(2);
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

fn frame(w: usize, h: usize, i: usize) -> Vec<u8> {
    let mut rng = Lcg(i as u64 * 7919 + 1);
    let mut f = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let base = ((x + i * 5) * 255 / w) as u32 / 2;
            let block = (((x / 16 + y / 16 + i) * 37) % 90) as u32;
            let grain = (rng.next() % 41) as i32 - 20;
            f[y * w + x] = (base as i32 + block as i32 + grain).clamp(0, 255) as u8;
        }
    }
    // hard-edged moving rectangles: sharp edges are what block codecs smear
    let (rw, rh) = (w / 8, h / 8);
    for (value, speed, row) in [(235u8, 13usize, h / 3), (20u8, 9usize, h / 2)] {
        let x0 = (i * speed) % (w - rw);
        for y in row..(row + rh).min(h) {
            f[y * w + x0..y * w + x0 + rw].fill(value);
        }
    }
    f
}

fn main() {
    let mut out: Option<PathBuf> = None;
    let (mut frames, mut size, mut fps) = (90usize, (1920usize, 1080usize), 30u32);
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--frames" => {
                frames = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| usage("--frames needs a number"))
            }
            "--fps" => {
                fps = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| usage("--fps needs a number"))
            }
            "--size" => {
                let v = it.next().unwrap_or_else(|| usage("--size needs WxH"));
                let (w, h) = v
                    .split_once('x')
                    .unwrap_or_else(|| usage("--size must look like 1920x1080"));
                size = (
                    w.parse()
                        .unwrap_or_else(|_| usage("--size must look like 1920x1080")),
                    h.parse()
                        .unwrap_or_else(|_| usage("--size must look like 1920x1080")),
                );
            }
            s if s.starts_with("--") => usage(&format!("unknown option {s}")),
            _ if out.is_none() => out = Some(PathBuf::from(a)),
            _ => usage("more than one output given"),
        }
    }
    let out = out.unwrap_or_else(|| usage(""));
    if frames == 0 || frames > 0xFF_FFFF {
        usage("--frames must be between 1 and 16777215");
    }
    if fps == 0 {
        usage("--fps must be positive");
    }
    let (w, h) = size;
    match strip_size(w) {
        Some((_, sh)) if h >= sh && w % 2 == 0 && h % 2 == 0 => {}
        Some(_) => usage("--size needs even dimensions and a height that fits the strip"),
        None => usage("--size is too narrow for the sync strip (at least 480 px wide)"),
    }
    if let Some(dir) = out.parent() {
        if !dir.as_os_str().is_empty() && !dir.is_dir() {
            eprintln!("error: output directory does not exist: {}", dir.display());
            std::process::exit(1);
        }
    }

    let mut child = match Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray",
            "-s",
        ])
        .arg(format!("{w}x{h}"))
        .args([
            "-r",
            &fps.to_string(),
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "10",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&out)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: could not run ffmpeg (is it installed and on PATH?): {e}");
            std::process::exit(1);
        }
    };
    let mut stdin = child.stdin.take().expect("stdin was piped");
    for i in 0..frames {
        let mut f = frame(w, h, i);
        stamp(&mut f, w, h, 1000 + (i / 3) as u32, i as u32).expect("size was validated above");
        if let Err(e) = stdin.write_all(&f) {
            drop(stdin);
            let _ = child.wait();
            eprintln!("error: ffmpeg stopped reading after {i} frames: {e}");
            std::process::exit(1);
        }
    }
    drop(stdin);
    match child.wait() {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("error: ffmpeg failed ({s})");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: waiting for ffmpeg: {e}");
            std::process::exit(1);
        }
    }
}
