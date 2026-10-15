//! Stage 1 evaluation: how well does "count pixels with fire colours" tell whether a frame shows fire?
//!
//! Usage: fire-presence VIDEO JOINED.jsonl [--stride N] [--dist D] [--assets DIR]
//! For each frame: score = pixels within D of the fire palette (sync strip rows skipped); truth = visible fire area
//! (sum over parts of box area x vis, video px^2) from the label. Prints, per truth-area bucket, how many frames reach
//! each score threshold.

use fire_core::json::{parse, Value};
use fire_core::presence::Palette;
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const STRIP_ROWS: usize = 16;
const BUCKETS: [&str; 6] = [
    "no fire",
    "area <100",
    "100-1k",
    "1k-10k",
    "10k-100k",
    ">=100k",
];
const THRESHOLDS: [usize; 8] = [10, 30, 100, 300, 1000, 3000, 10000, 30000];

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn bucket(area: f64, fire: bool) -> usize {
    if !fire {
        0
    } else if area < 100.0 {
        1
    } else if area < 1_000.0 {
        2
    } else if area < 10_000.0 {
        3
    } else if area < 100_000.0 {
        4
    } else {
        5
    }
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut stride = 1u64;
    let mut dist = 24.0f64;
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
            "--dist" => {
                dist = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| die("--dist needs a number".into()))
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
        die("usage: fire-presence VIDEO JOINED.jsonl [--stride N] [--dist D] [--assets DIR]".into())
    };
    let s0 = std::fs::read(format!("{assets}/fire_0.rgba"))
        .unwrap_or_else(|e| die(format!("fire_0.rgba: {e}")));
    let s1 = std::fs::read(format!("{assets}/fire_1.rgba"))
        .unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));
    let palette = Palette::from_rgba(&[&s0, &s1], dist).unwrap_or_else(|e| die(e));

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
    let mut scores: Vec<Vec<usize>> = vec![Vec::new(); BUCKETS.len()];
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
        let (Some(lw), Some(fire)) = (
            label.get("w").and_then(Value::num),
            label.get("fire").and_then(Value::bool),
        ) else {
            continue;
        };
        let scale = info.width as f64 / lw;
        let mut area = 0.0;
        if let Some(fires) = label.get("fires").and_then(Value::arr) {
            for f in fires {
                for p in f.get("parts").and_then(Value::arr).unwrap_or(&[]) {
                    let b: Vec<f64> = p
                        .get("box")
                        .and_then(Value::arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(Value::num)
                        .collect();
                    let vis = p.get("vis").and_then(Value::num).unwrap_or(0.0);
                    if b.len() == 4 {
                        area += (b[2] - b[0]) * scale * (b[3] - b[1]) * scale * vis;
                    }
                }
            }
        }
        let score = palette
            .count_rows(&frame, info.width, info.height, STRIP_ROWS, info.height)
            .unwrap_or_else(|e| die(e));
        scores[bucket(area, fire)].push(score);
    }
    let _ = child.kill();
    let _ = child.wait();

    println!(
        "fire-presence: frames={n} stride={stride} dist={dist} video={}x{}",
        info.width, info.height
    );
    print!("{:<12} {:>7} {:>9}", "truth", "frames", "median");
    for t in THRESHOLDS {
        print!(" {:>7}", format!(">={t}"));
    }
    println!();
    for (i, name) in BUCKETS.iter().enumerate() {
        let s = &mut scores[i];
        if s.is_empty() {
            continue;
        }
        s.sort_unstable();
        print!("{:<12} {:>7} {:>9}", name, s.len(), s[s.len() / 2]);
        for t in THRESHOLDS {
            let hit = s.iter().filter(|&&v| v >= t).count();
            print!(" {:>6.1}%", 100.0 * hit as f64 / s.len() as f64);
        }
        println!();
    }
}
