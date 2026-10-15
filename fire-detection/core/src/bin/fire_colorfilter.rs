//! Pure colour filter experiment: for several palette tolerances D and opening radii r, the surviving pixels of the
//! palette mask after outlier removal, per frame, next to the label truth.
//!
//! Usage: fire-colorfilter VIDEO JOINED.jsonl --out colorfilter.csv [--stride N] [--assets DIR]
//! Columns: n,fire,area,mind,veil, then for each (D, r): c<D>_<r>_tot (surviving px), c<D>_<r>_big (largest blob px).

use fire_core::features::{blob_stats, open_mask};
use fire_core::json::{parse, Value};
use fire_core::presence::Palette;
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

const STRIP_ROWS: usize = 16;
const DISTS: [f64; 3] = [12.0, 24.0, 40.0];
const RADII: [usize; 3] = [0, 1, 2];

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut out = None;
    let mut stride = 10u64;
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
        die("usage: fire-colorfilter VIDEO JOINED.jsonl --out colorfilter.csv [--stride N] [--assets DIR]".into())
    };
    let s0 = std::fs::read(format!("{assets}/fire_0.rgba"))
        .unwrap_or_else(|e| die(format!("fire_0.rgba: {e}")));
    let s1 = std::fs::read(format!("{assets}/fire_1.rgba"))
        .unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));
    let palettes: Vec<Palette> = DISTS
        .iter()
        .map(|&d| Palette::from_rgba(&[&s0, &s1], d).unwrap_or_else(|e| die(e)))
        .collect();

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
    let mut header = "n,fire,area,mind,veil".to_string();
    for d in DISTS {
        for r in RADII {
            header.push_str(&format!(",c{}_{}_tot,c{}_{}_big", d as u32, r, d as u32, r));
        }
    }
    writeln!(csv, "{header}").unwrap();

    let rows =
        BufReader::new(File::open(&joined).unwrap_or_else(|e| die(format!("{joined}: {e}"))));
    let (mut n, mut written) = (0u64, 0u64);
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
        let (Some(lw), Some(fire)) = (
            label.get("w").and_then(Value::num),
            label.get("fire").and_then(Value::bool),
        ) else {
            continue;
        };
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
                    veil = veil.max(p.get("veil").and_then(Value::num).unwrap_or(0.0));
                }
            }
        }
        let mut line = format!(
            "{this},{},{:.1},{},{:.3}",
            fire as u8,
            area,
            if mind.is_finite() {
                format!("{mind:.2}")
            } else {
                String::new()
            },
            veil
        );
        for pal in &palettes {
            let mask = pal
                .mask(&frame, w, h, STRIP_ROWS, h)
                .unwrap_or_else(|e| die(e));
            for r in RADII {
                let m = if r == 0 {
                    mask.clone()
                } else {
                    open_mask(&mask, w, rows_h, r).unwrap_or_else(|e| die(e))
                };
                let b = blob_stats(&m, w, rows_h, 1).unwrap_or_else(|e| die(e));
                line.push_str(&format!(",{},{}", b.total, b.largest));
            }
        }
        writeln!(csv, "{line}").unwrap();
        written += 1;
    }
    let _ = child.kill();
    let _ = child.wait();
    csv.flush().unwrap();
    println!("fire-colorfilter: frames={n} written={written} stride={stride} -> {out}");
}
