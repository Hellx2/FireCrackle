//! Sprite matching of the first-person on-fire overlay, for several guessed FOVs, next to the label truth.
//!
//! Usage: fire-overlay VIDEO JOINED.jsonl --out overlay.csv [--stride N] [--fovs 60,70,...] [--assets DIR]
//! Columns: n,has_overlay,truth_row,vis,veil, then per FOV f: row<f>,z<f>,r<f>,n<f> (best row, its z, correlation, samples).
//! truth_row/vis/veil are -1/0/0 when the label has no overlay source.

use fire_core::features::unveil_rows;
use fire_core::json::{parse, Value};
use fire_core::overlay::{match_overlay, SilhouetteMatcher};
use fire_core::veil::{detect_veil, A_BOTTOM, A_TOP, COLOUR};
use fire_core::video::{parse_probe, read_frame};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

fn die(msg: String) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut video = None;
    let mut joined = None;
    let mut out = None;
    let mut stride = 5u64;
    let mut fovs: Vec<f64> = vec![60.0, 70.0, 80.0, 90.0, 100.0, 110.0];
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
            "--fovs" => {
                fovs = it
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .map(|v| {
                        v.trim()
                            .parse()
                            .unwrap_or_else(|_| die(format!("bad fov {v}")))
                    })
                    .collect()
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
        die("usage: fire-overlay VIDEO JOINED.jsonl --out overlay.csv [--stride N] [--fovs 60,70,...]".into())
    };
    let sheet1 = std::fs::read(format!("{assets}/fire_1.rgba"))
        .unwrap_or_else(|e| die(format!("fire_1.rgba: {e}")));

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
    let mut frame = vec![0u8; w * h * 3];

    let silhouette = SilhouetteMatcher::new(&sheet1, w, h, 70.0).unwrap_or_else(|e| die(e));
    let silhouette60 = SilhouetteMatcher::new(&sheet1, w, h, 60.0).unwrap_or_else(|e| die(e));
    let mut csv = BufWriter::new(File::create(&out).unwrap_or_else(|e| die(format!("{out}: {e}"))));
    let mut header = "n,has_overlay,truth_row,vis,veil".to_string();
    for f in &fovs {
        header.push_str(&format!(",row{0},z{0},r{0},n{0}", *f as u32));
    }
    header.push_str(",sil_row,sil_agree,sil_pres,col_row,col_score,veiled,uv_row,uv_pres,uv_col,f60_row,f60_pres,f60_col,fin_row,fin_fov,fin_ev,fin_pres");
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
        if label.get("w").is_none() {
            continue;
        }
        let (mut has, mut truth_row, mut vis, mut veil) = (0, -1i64, 0.0f64, 0.0f64);
        for f in label.get("fires").and_then(Value::arr).unwrap_or(&[]) {
            if f.get("src").and_then(Value::str) == Some("overlay") {
                has = 1;
                vis = f.get("vis").and_then(Value::num).unwrap_or(0.0);
                veil = f.get("veil").and_then(Value::num).unwrap_or(0.0);
                if let Some(p) = f.get("parts").and_then(Value::arr).and_then(|p| p.first()) {
                    truth_row = p
                        .get("row")
                        .and_then(Value::num)
                        .map(|r| r as i64)
                        .unwrap_or(-1);
                }
            }
        }
        let mut line = format!("{this},{has},{truth_row},{vis:.3},{veil:.3}");
        for f in &fovs {
            let r = match_overlay(&sheet1, &frame, w, h, *f).unwrap_or_else(|e| die(e));
            line.push_str(&format!(
                ",{},{:.2},{:.3},{}",
                r[0].row, r[0].z, r[0].score, r[0].samples
            ));
        }
        let sr = silhouette.match_frame(&frame).unwrap_or_else(|e| die(e));
        // the GUI background veil (pause menu, inventory) dims everything: detect it and undo it, then match again
        let veiled = detect_veil(&frame, w, h, 16).unwrap_or_else(|e| die(e));
        let cleaned: Option<Vec<u8>> = if veiled {
            Some(
                unveil_rows(&frame, w, h, A_TOP, A_BOTTOM, [COLOUR as u8; 3])
                    .unwrap_or_else(|e| die(e)),
            )
        } else {
            None
        };
        let src: &[u8] = cleaned.as_deref().unwrap_or(&frame);
        let r70 = silhouette.match_frame(src).unwrap_or_else(|e| die(e));
        // the overlay FOV is 60 instead of 70 while the camera is submerged in a fluid (lava): try that geometry too
        let r60 = silhouette60.match_frame(src).unwrap_or_else(|e| die(e));
        line.push_str(&format!(
            ",{},{:.4},{:.4},{},{:.3},{},{},{:.4},{:.3},{},{:.4},{:.3}",
            sr.ranked[0].0,
            sr.ranked[0].1,
            sr.presence,
            sr.ranked_colour[0].0,
            sr.ranked_colour[0].1,
            veiled as u8,
            r70.ranked_colour[0].0,
            r70.presence,
            r70.ranked_colour[0].1,
            r60.ranked_colour[0].0,
            r60.presence,
            r60.ranked_colour[0].1
        ));
        // final answer: the FOV (70, or 60 while submerged) whose geometry explains the colours better
        let (fin, fov) = if r60.colour_evidence > r70.colour_evidence {
            (&r60, 60)
        } else {
            (&r70, 70)
        };
        line.push_str(&format!(
            ",{},{},{:.4},{:.4}",
            fin.ranked_colour[0].0, fov, fin.colour_evidence, fin.presence
        ));
        writeln!(csv, "{line}").unwrap();
        written += 1;
    }
    let _ = child.kill();
    let _ = child.wait();
    csv.flush().unwrap();
    println!("fire-overlay: frames={n} written={written} stride={stride} -> {out}");
}
