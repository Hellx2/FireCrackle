//! Dumps labelled patch features for training and for evaluating the patch detector.
//!
//! Usage: fire-patches VIDEO JOINED.jsonl --out patches.csv [--stride N] [--neg-keep K] [--assets DIR]
//! One row per kept patch: n,px,py,truth(1/0),<12 features>. Positives are all kept, negatives 1 in K (deterministic),
//! ambiguous patches are skipped. Rows start below the 16-px sync strip.
//!
//! With `--model tree.json --frames-out frames.csv` it instead scores EVERY patch of each sampled frame with the tree and
//! writes one row per frame: n,fire,area,mind,veil,positives(p>=0.5),largest_cluster,max_p.

use fire_core::json::{parse, Value};
use fire_core::patch::{frame_patch_features, patch_truth, pool_patches, PatchModel, PatchTruth, FEATURE_NAMES, PATCH};
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

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut out = None;
    let mut model_path = None;
    let mut frames_out = None;
    let mut stride = 20u64;
    let mut neg_keep = 20u64;
    let mut min_n = 0u64;
    let mut assets = "data/assets".to_string();
    let mut it = std::env::args().skip(1);
    let num = |v: Option<String>, what: &str| -> u64 {
        v.and_then(|v| v.parse().ok()).filter(|&s| s > 0).unwrap_or_else(|| die(format!("{what} needs a positive number")))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--stride" => stride = num(it.next(), "--stride"),
            "--neg-keep" => neg_keep = num(it.next(), "--neg-keep"),
            "--min-n" => min_n = it.next().and_then(|v| v.parse().ok()).unwrap_or_else(|| die("--min-n needs a number".into())),
            "--out" => out = it.next(),
            "--model" => model_path = it.next(),
            "--frames-out" => frames_out = it.next(),
            "--assets" => assets = it.next().unwrap_or_else(|| die("--assets needs a directory".into())),
            _ if video.is_none() => video = Some(a),
            _ if joined.is_none() => joined = Some(a),
            _ => die(format!("unexpected argument {a}")),
        }
    }
    let (Some(video), Some(joined)) = (video, joined) else {
        die("usage: fire-patches VIDEO JOINED.jsonl --out patches.csv | --model tree.json --frames-out frames.csv".into())
    };
    let scoring = model_path.is_some();
    if scoring && frames_out.is_none() || !scoring && out.is_none() {
        die("give --out (training rows) or --model with --frames-out (frame scores)".into());
    }
    let s0 = std::fs::read(format!("{assets}/fire_0.rgba")).unwrap_or_else(|e| die(format!("fire_0.rgba: {e}")));
    let s1 = std::fs::read(format!("{assets}/fire_1.rgba")).unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));
    let loose = Palette::from_rgba(&[&s0, &s1], 24.0).unwrap_or_else(|e| die(e));
    let strict = Palette::from_rgba(&[&s0, &s1], 8.0).unwrap_or_else(|e| die(e));
    let model = model_path.map(|p| {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| die(format!("{p}: {e}")));
        PatchModel::from_json(&text).unwrap_or_else(|e| die(format!("{p}: {e}")))
    });

    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,r_frame_rate"])
        .args(["-of", "default=noprint_wrappers=1"])
        .arg(&video)
        .output()
        .unwrap_or_else(|e| die(format!("ffprobe: {e}")));
    let info = parse_probe(&String::from_utf8_lossy(&probe.stdout)).unwrap_or_else(|e| die(e));
    let mut child = Command::new("ffmpeg")
        .args(["-nostdin", "-loglevel", "error", "-nostats", "-i"])
        .arg(&video)
        .args(["-fps_mode", "passthrough", "-pix_fmt", "rgb24", "-f", "rawvideo", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die(format!("ffmpeg: {e}")));
    let mut pipe = child.stdout.take().expect("stdout was piped");
    let (w, h) = (info.width, info.height);
    let mut frame = vec![0u8; w * h * 3];

    let target = if scoring { frames_out.clone().unwrap() } else { out.clone().unwrap() };
    let mut csv = BufWriter::new(File::create(&target).unwrap_or_else(|e| die(format!("{target}: {e}"))));
    if scoring {
        writeln!(csv, "n,fire,area,mind,veil,positives,largest,max_p").unwrap();
    } else {
        writeln!(csv, "n,px,py,truth,{}", FEATURE_NAMES.join(",")).unwrap();
    }

    let rows = BufReader::new(File::open(&joined).unwrap_or_else(|e| die(format!("{joined}: {e}"))));
    let mut rng = 0x2545F4914F6CDD1Du64;
    let (mut n, mut pos_rows, mut neg_rows, mut frames) = (0u64, 0u64, 0u64, 0u64);
    for line in rows.lines() {
        let line = line.unwrap_or_else(|e| die(format!("reading {joined}: {e}")));
        match read_frame(&mut pipe, &mut frame) {
            Ok(true) => {}
            Ok(false) => die(format!("video ended at frame {n} but {joined} has more rows")),
            Err(e) => die(format!("reading video: {e}")),
        }
        let this = n;
        n += 1;
        if this % stride != 0 || this < min_n {
            continue;
        }
        let Ok(row) = parse(&line) else { continue };
        let Some(label) = row.get("label") else { continue };
        let (Some(lw), Some(fire)) = (label.get("w").and_then(Value::num), label.get("fire").and_then(Value::bool)) else {
            continue;
        };
        let scale = w as f64 / lw;
        let mut boxes: Vec<([f64; 4], f64)> = Vec::new();
        let (mut area, mut mind, mut veil) = (0.0f64, f64::INFINITY, 0.0f64);
        for f in label.get("fires").and_then(Value::arr).unwrap_or(&[]) {
            let d = f.get("dist").and_then(Value::num).unwrap_or(0.0);
            for p in f.get("parts").and_then(Value::arr).unwrap_or(&[]) {
                let b: Vec<f64> = p.get("box").and_then(Value::arr).unwrap_or(&[]).iter().filter_map(Value::num).collect();
                let vis = p.get("vis").and_then(Value::num).unwrap_or(0.0);
                if b.len() == 4 {
                    let bx = [b[0] * scale, b[1] * scale, b[2] * scale, b[3] * scale];
                    area += (bx[2] - bx[0]) * (bx[3] - bx[1]) * vis;
                    mind = mind.min(d);
                    veil = veil.max(p.get("veil").and_then(Value::num).unwrap_or(0.0));
                    boxes.push((bx, vis));
                }
            }
        }
        let grid = frame_patch_features(&frame, w, h, STRIP_ROWS, &loose, &strict).unwrap_or_else(|e| die(e));
        frames += 1;
        if let Some(m) = &model {
            let probs: Vec<f32> = grid.feats.iter().map(|f| m.predict(f)).collect();
            let positive: Vec<bool> = probs.iter().map(|&p| p >= 0.5).collect();
            let pooled = pool_patches(&positive, grid.gx, grid.gy).unwrap_or_else(|e| die(e));
            let max_p = probs.iter().cloned().fold(0.0f32, f32::max);
            writeln!(
                csv,
                "{this},{},{:.1},{},{:.3},{},{},{:.3}",
                fire as u8,
                area,
                if mind.is_finite() { format!("{mind:.2}") } else { String::new() },
                veil,
                pooled.positives,
                pooled.largest_cluster,
                max_p
            )
            .unwrap();
            continue;
        }
        for gy in 0..grid.gy {
            for gx in 0..grid.gx {
                let (px, py) = (gx * PATCH, STRIP_ROWS + gy * PATCH);
                let truth = match patch_truth(&boxes, px, py) {
                    PatchTruth::Pos => 1,
                    PatchTruth::Neg => {
                        rng ^= rng << 13;
                        rng ^= rng >> 7;
                        rng ^= rng << 17;
                        if rng % neg_keep != 0 {
                            continue;
                        }
                        0
                    }
                    PatchTruth::Skip => continue,
                };
                if truth == 1 {
                    pos_rows += 1;
                } else {
                    neg_rows += 1;
                }
                let f = grid.feats[gy * grid.gx + gx];
                let fs: Vec<String> = f.iter().map(|v| format!("{v:.4}")).collect();
                writeln!(csv, "{this},{px},{py},{truth},{}", fs.join(",")).unwrap();
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    csv.flush().unwrap();
    println!("fire-patches: frames={n} used={frames} pos_rows={pos_rows} neg_rows={neg_rows} -> {target}");
}
