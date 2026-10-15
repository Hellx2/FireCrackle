//! Per-blob shape features of the cleaned palette mask, with blob truth from the labels.
//!
//! Usage: fire-blobs VIDEO JOINED.jsonl --out blobs.csv [--stride N] [--min-area A] [--dist D] [--radius R] [--assets DIR]
//! Rows: n,area,bw,bh,fill,compact,lum,lumstd,white,sat,rough,truth
//!   fill = area / bbox area, compact = perimeter / sqrt(area), lum/lumstd/white/sat over the blob's pixels (0..1),
//!   rough = mean |dluma| between horizontal neighbours both in the blob (0..1),
//!   truth = 1 if >= 50% of the blob lies in a part box with vis >= 0.3, 0 if none of it lies in any box, -1 otherwise.

use fire_core::features::{label_blobs, open_mask};
use fire_core::json::{parse, Value};
use fire_core::presence::Palette;
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

const STRIP_ROWS: usize = 16;

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

#[derive(Clone, Copy, Default)]
struct Acc {
    n_pos_box: usize,
    n_any_box: usize,
    sum_l: f64,
    sum_l2: f64,
    white: usize,
    sat: f64,
    dx: f64,
    ndx: usize,
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut out = None;
    let mut frames_out: Option<String> = None;
    let mut stride = 10u64;
    let mut min_area = 100usize;
    let mut dist = 24.0f64;
    let mut radius = 2usize;
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
            "--min-area" => {
                min_area = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| die("--min-area needs a number".into()))
            }
            "--dist" => {
                dist = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| die("--dist needs a number".into()))
            }
            "--radius" => {
                radius = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| die("--radius needs a number".into()))
            }
            "--out" => out = it.next(),
            "--frames-out" => frames_out = it.next(),
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
        die("usage: fire-blobs VIDEO JOINED.jsonl --out blobs.csv [--stride N] [--min-area A] [--dist D] [--radius R]".into())
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
    let (w, h) = (info.width, info.height);
    let rows_h = h - STRIP_ROWS;
    let mut frame = vec![0u8; w * h * 3];

    let mut csv = BufWriter::new(File::create(&out).unwrap_or_else(|e| die(format!("{out}: {e}"))));
    writeln!(
        csv,
        "n,area,bw,bh,fill,compact,lum,lumstd,white,sat,rough,truth"
    )
    .unwrap();
    // optional per-frame truth: n,fire,area,mind,veil (same columns as fire-colorfilter)
    let mut fcsv = frames_out.as_ref().map(|p| {
        let mut f = BufWriter::new(File::create(p).unwrap_or_else(|e| die(format!("{p}: {e}"))));
        writeln!(f, "n,fire,area,mind,veil").unwrap();
        f
    });

    let rows =
        BufReader::new(File::open(&joined).unwrap_or_else(|e| die(format!("{joined}: {e}"))));
    let (mut n, mut frames, mut blobs_out) = (0u64, 0u64, 0u64);
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
        if this % stride != 0 {
            continue;
        }
        let Ok(row) = parse(&line) else { continue };
        let Some(label) = row.get("label") else {
            continue;
        };
        let Some(lw) = label.get("w").and_then(Value::num) else {
            continue;
        };
        let scale = w as f64 / lw;
        let mut boxes: Vec<([f64; 4], bool)> = Vec::new(); // (box in video px, vis >= 0.3)
        let (mut f_area, mut f_mind, mut f_veil) = (0.0f64, f64::INFINITY, 0.0f64);
        for f in label.get("fires").and_then(Value::arr).unwrap_or(&[]) {
            let fdist = f.get("dist").and_then(Value::num).unwrap_or(0.0);
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
                    let bx = [b[0] * scale, b[1] * scale, b[2] * scale, b[3] * scale];
                    f_area += (bx[2] - bx[0]) * (bx[3] - bx[1]) * vis;
                    f_mind = f_mind.min(fdist);
                    f_veil = f_veil.max(p.get("veil").and_then(Value::num).unwrap_or(0.0));
                    boxes.push((bx, vis >= 0.3));
                }
            }
        }
        frames += 1;
        if let Some(f) = fcsv.as_mut() {
            let fire = label.get("fire").and_then(Value::bool).unwrap_or(false);
            writeln!(
                f,
                "{this},{},{:.1},{},{:.3}",
                fire as u8,
                f_area,
                if f_mind.is_finite() {
                    format!("{f_mind:.2}")
                } else {
                    String::new()
                },
                f_veil
            )
            .unwrap();
        }
        let mask = palette
            .mask(&frame, w, h, STRIP_ROWS, h)
            .unwrap_or_else(|e| die(e));
        let mask = open_mask(&mask, w, rows_h, radius).unwrap_or_else(|e| die(e));
        let (labels, infos) = label_blobs(&mask, w, rows_h).unwrap_or_else(|e| die(e));
        let mut acc = vec![Acc::default(); infos.len()];
        for (i, &l) in labels.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let a = &mut acc[l as usize - 1];
            let (x, y) = (i % w, i / w + STRIP_ROWS);
            let o = (y * w + x) * 3;
            let (r, g, b) = (frame[o] as f64, frame[o + 1] as f64, frame[o + 2] as f64);
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            a.sum_l += lum;
            a.sum_l2 += lum * lum;
            a.white += (lum >= 235.0) as usize;
            let mx = r.max(g).max(b);
            if mx > 0.0 {
                a.sat += (mx - r.min(g).min(b)) / mx;
            }
            if x + 1 < w && labels[i + 1] == l {
                let o2 = o + 3;
                let lum2 = 0.299 * frame[o2] as f64
                    + 0.587 * frame[o2 + 1] as f64
                    + 0.114 * frame[o2 + 2] as f64;
                a.dx += (lum - lum2).abs();
                a.ndx += 1;
            }
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            for (bx, strong) in &boxes {
                if fx >= bx[0] && fx < bx[2] && fy >= bx[1] && fy < bx[3] {
                    a.n_any_box += 1;
                    if *strong {
                        a.n_pos_box += 1;
                        break;
                    }
                }
            }
        }
        for (info, a) in infos.iter().zip(&acc) {
            if info.area < min_area {
                continue;
            }
            let area = info.area as f64;
            let (bw, bh) = ((info.x1 - info.x0) as f64, (info.y1 - info.y0) as f64);
            let mean = a.sum_l / area;
            let std = (a.sum_l2 / area - mean * mean).max(0.0).sqrt();
            let truth = if a.n_pos_box as f64 >= 0.5 * area {
                1
            } else if a.n_any_box == 0 {
                0
            } else {
                -1
            };
            writeln!(
                csv,
                "{this},{},{},{},{:.4},{:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{truth}",
                info.area,
                bw,
                bh,
                area / (bw * bh),
                info.perimeter as f64 / area.sqrt(),
                mean / 255.0,
                std / 255.0,
                a.white as f64 / area,
                a.sat / area,
                if a.ndx > 0 {
                    a.dx / a.ndx as f64 / 255.0
                } else {
                    0.0
                }
            )
            .unwrap();
            blobs_out += 1;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    csv.flush().unwrap();
    if let Some(f) = fcsv.as_mut() {
        f.flush().unwrap();
    }
    println!("fire-blobs: frames={n} used={frames} blobs={blobs_out} -> {out}");
}
