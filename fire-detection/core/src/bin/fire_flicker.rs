//! Temporal flicker features per sampled frame, next to the label truth.
//!
//! Usage: fire-flicker VIDEO JOINED.jsonl --out flicker.csv [--stride N] [--assets DIR]
//! Each sampled frame B (tick k, tick_ok) is paired with the latest earlier frame A of tick k-1.
//! Columns: n,fire,area,mind,veil,gap,bg, then for luma-change thresholds 30 and 60: t<T>_tot (flicker px),
//! t<T>_in6 (flicker px in 4-connected blobs >= 6 px), t<T>_big (largest flicker blob).
//! gap = video frames between A and B; bg = mean |dluma| of non-fire pixels (camera motion).

use fire_core::features::blob_stats;
use fire_core::flicker::{bg_motion, find_partner, flicker_mask};
use fire_core::json::{parse, Value};
use fire_core::presence::Palette;
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

const STRIP_ROWS: usize = 16;
const KEEP: usize = 8; // frames of history (about 2-3 ticks at ~3 frames per tick)
const THRESHOLDS: [f64; 2] = [30.0, 60.0];

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut out = None;
    let mut stride = 5u64;
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
            "--out" => out = it.next(),
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
    let (Some(video), Some(joined), Some(out)) = (video, joined, out) else {
        die(
            "usage: fire-flicker VIDEO JOINED.jsonl --out flicker.csv [--stride N] [--assets DIR]"
                .into(),
        )
    };
    let s0 = std::fs::read(format!("{assets}/fire_0.rgba"))
        .unwrap_or_else(|e| die(format!("fire_0.rgba: {e}")));
    let s1 = std::fs::read(format!("{assets}/fire_1.rgba"))
        .unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));
    let palette = Palette::from_rgba(&[&s0, &s1], 24.0).unwrap_or_else(|e| die(e));

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
            fire_core::video::PASSTHROUGH_ARGS[0],
            fire_core::video::PASSTHROUGH_ARGS[1],
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
    let (w, h) = (info.width, info.height);
    let rows_h = h - STRIP_ROWS;

    let mut csv = BufWriter::new(File::create(&out).unwrap_or_else(|e| die(format!("{out}: {e}"))));
    let mut header = "n,fire,area,mind,veil,gap,bg".to_string();
    for t in THRESHOLDS {
        header.push_str(&format!(",t{0}_tot,t{0}_in6,t{0}_big", t as u32));
    }
    writeln!(csv, "{header}").unwrap();

    // ring of the last KEEP frames: (frame number, tick, pixels)
    let mut ring: Vec<(u64, u64, Vec<u8>)> = Vec::new();
    let rows =
        BufReader::new(File::open(&joined).unwrap_or_else(|e| die(format!("{joined}: {e}"))));
    let (mut n, mut written, mut no_partner) = (0u64, 0u64, 0u64);
    for line in rows.lines() {
        let line = line.unwrap_or_else(|e| die(format!("reading {joined}: {e}")));
        let mut buf = if ring.len() == KEEP {
            ring.remove(0).2
        } else {
            vec![0u8; w * h * 3]
        };
        match read_frame(&mut pipe, &mut buf) {
            Ok(true) => {}
            Ok(false) => die(format!(
                "video ended at frame {n} but {joined} has more rows"
            )),
            Err(e) => die(format!("reading video: {e}")),
        }
        let this = n;
        n += 1;
        let row = parse(&line).ok();
        let tick = row
            .as_ref()
            .filter(|r| r.get("tick_ok").and_then(Value::bool).unwrap_or(false))
            .and_then(|r| r.get("tick").and_then(Value::num))
            .map(|t| t as u64);
        // the current frame goes into the history AFTER the pair is formed; frames without a trusted tick use u64::MAX
        let cur_tick = tick.unwrap_or(u64::MAX);
        let pair = if this % stride == 0 && tick.is_some() {
            let hist: Vec<(u64, u64)> = ring.iter().map(|(f, t, _)| (*f, *t)).collect();
            find_partner(&hist, cur_tick).map(|i| (i, this - ring[i].0))
        } else {
            None
        };
        if this % stride == 0 {
            if let (Some(row), Some((pi, gap))) = (row.as_ref(), pair) {
                if let Some(label) = row.get("label") {
                    if let (Some(lw), Some(fire)) = (
                        label.get("w").and_then(Value::num),
                        label.get("fire").and_then(Value::bool),
                    ) {
                        let scale = w as f64 / lw;
                        let (mut area, mut mind, mut veil) = (0.0f64, f64::INFINITY, 0.0f64);
                        for f in label.get("fires").and_then(Value::arr).unwrap_or(&[]) {
                            let d = f.get("dist").and_then(Value::num).unwrap_or(0.0);
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
                                    mind = mind.min(d);
                                    veil =
                                        veil.max(p.get("veil").and_then(Value::num).unwrap_or(0.0));
                                }
                            }
                        }
                        let a = &ring[pi].2;
                        let bg = bg_motion(a, &buf, w, h, STRIP_ROWS, &palette, 4)
                            .unwrap_or_else(|e| die(e));
                        let mut line = format!(
                            "{this},{},{:.1},{},{:.3},{gap},{:.2}",
                            fire as u8,
                            area,
                            if mind.is_finite() {
                                format!("{mind:.2}")
                            } else {
                                String::new()
                            },
                            veil,
                            bg
                        );
                        for t in THRESHOLDS {
                            let m = flicker_mask(a, &buf, w, h, STRIP_ROWS, &palette, t)
                                .unwrap_or_else(|e| die(e));
                            let tot = m.iter().filter(|&&x| x).count();
                            let b6 = blob_stats(&m, w, rows_h, 6).unwrap_or_else(|e| die(e));
                            line.push_str(&format!(",{tot},{},{}", b6.in_big, b6.largest));
                        }
                        writeln!(csv, "{line}").unwrap();
                        written += 1;
                    }
                }
            }
        }
        if this % stride == 0 && pair.is_none() {
            no_partner += 1;
        }
        ring.push((this, cur_tick, buf));
    }
    let _ = child.kill();
    let _ = child.wait();
    csv.flush().unwrap();
    println!("fire-flicker: frames={n} written={written} without_partner={no_partner} stride={stride} -> {out}");
}
