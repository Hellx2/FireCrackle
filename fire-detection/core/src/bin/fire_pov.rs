//! Video-only vanilla 1.16.5 first-person overlay detection. No mod/labels required.

use fire_core::features::unveil_rows;
use fire_core::overlay::SilhouetteMatcher;
use fire_core::veil::{detect_veil, A_BOTTOM, A_TOP, COLOUR};
use fire_core::video::{parse_probe, read_frame};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

fn accepted_row(row: u8, evidence: f64, threshold: f64) -> Option<u8> {
    (row < 32 && evidence.is_finite() && threshold.is_finite() && evidence >= threshold)
        .then_some(row)
}

// Reap ffmpeg even when matching, output writing or decoding fails midway.
struct Decoder(Child);
impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run() -> Result<(), String> {
    let mut video = None;
    let mut assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/assets");
    let (mut stride, mut limit) = (1u64, None);
    let mut threshold = 0.5f64;
    let (mut ffmpeg, mut ffprobe) = ("ffmpeg".to_string(), "ffprobe".to_string());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            eprintln!("fire_pov VIDEO [--stride N] [--limit N] [--threshold 0.5] [--assets DIR] [--ffmpeg PATH] [--ffprobe PATH]\nJSONL on stdout; row=null when evidence is below threshold. Vanilla 1.16.5, uncropped game viewport only.");
            return Ok(());
        }
        if arg.starts_with('-') {
            let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
            match arg.as_str() {
                "--stride" => {
                    stride = value
                        .parse()
                        .ok()
                        .filter(|&v| v > 0)
                        .ok_or("stride must be positive")?
                }
                "--limit" => {
                    limit = Some(
                        value
                            .parse::<u64>()
                            .ok()
                            .filter(|&v| v > 0)
                            .ok_or("limit must be positive")?,
                    )
                }
                "--threshold" => {
                    threshold = value.parse().map_err(|_| "threshold must be a number")?;
                    if !threshold.is_finite() || !(-1.0..=1.0).contains(&threshold) {
                        return Err("threshold must be finite and in [-1, 1]".into());
                    }
                }
                "--assets" => assets = value.into(),
                "--ffmpeg" => ffmpeg = value,
                "--ffprobe" => ffprobe = value,
                _ => return Err(format!("unknown option {arg}")),
            }
        } else if video.replace(arg).is_some() {
            return Err("only one video may be supplied".into());
        }
    }
    let video = video.ok_or("usage: fire_pov VIDEO [--help]")?;
    let sheet =
        std::fs::read(assets.join("fire_1.rgba")).map_err(|e| format!("fire_1.rgba: {e}"))?;
    let probe = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(&video)
        .output()
        .map_err(|e| format!("ffprobe: {e}"))?;
    if !probe.status.success() {
        return Err(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&probe.stderr)
        ));
    }
    let info = parse_probe(&String::from_utf8_lossy(&probe.stdout))?;
    let (w, h) = (info.width, info.height);
    let matcher70 = SilhouetteMatcher::new(&sheet, w, h, 70.0)?;
    let matcher60 = SilhouetteMatcher::new(&sheet, w, h, 60.0)?;
    let bytes = w
        .checked_mul(h)
        .and_then(|v| v.checked_mul(3))
        .ok_or("video dimensions overflow")?;
    let mut frame = vec![0u8; bytes];
    let mut decoder = Decoder(
        Command::new(ffmpeg)
            .args(["-nostdin", "-v", "error", "-i"])
            .arg(&video)
            .args([
                "-map",
                "0:v:0",
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
            .map_err(|e| format!("ffmpeg: {e}"))?,
    );
    let mut pipe = decoder.0.stdout.take().ok_or("ffmpeg stdout missing")?;
    let mut out = BufWriter::new(io::stdout().lock());
    let (mut n, mut emitted) = (0u64, 0u64);
    let mut limited = false;
    while read_frame(&mut pipe, &mut frame).map_err(|e| format!("frame {n}: {e}"))? {
        let current = n;
        n += 1;
        if current % stride != 0 {
            continue;
        }
        // Ignore the top 16 rows consistently with the existing benchmark (also excludes its sync strip).
        let veiled = detect_veil(&frame, w, h, 16.min(h - 1))?;
        let cleaned = if veiled {
            Some(unveil_rows(
                &frame,
                w,
                h,
                A_TOP,
                A_BOTTOM,
                [COLOUR as u8; 3],
            )?)
        } else {
            None
        };
        let src = cleaned.as_deref().unwrap_or(&frame);
        let r70 = matcher70.match_frame(src)?;
        let r60 = matcher60.match_frame(src)?;
        let (result, fov) = if r60.colour_evidence > r70.colour_evidence {
            (&r60, 60)
        } else {
            (&r70, 70)
        };
        let candidate = result.ranked_colour[0].0;
        let row = accepted_row(candidate, result.colour_evidence, threshold);
        let row_json = row.map_or_else(|| "null".to_string(), |r| r.to_string());
        writeln!(out, "{{\"n\":{current},\"detected\":{},\"row\":{row_json},\"candidate_row\":{candidate},\"evidence\":{:.6},\"fov\":{fov},\"veiled\":{veiled}}}", row.is_some(), result.colour_evidence)
            .map_err(|e| format!("writing output: {e}"))?;
        emitted += 1;
        if limit.is_some_and(|max| emitted >= max) {
            limited = true;
            break;
        }
    }
    if !limited {
        let status = decoder
            .0
            .wait()
            .map_err(|e| format!("waiting for ffmpeg: {e}"))?;
        if !status.success() {
            return Err(format!("ffmpeg failed: {status}"));
        }
    }
    out.flush().map_err(|e| format!("flushing output: {e}"))?;
    eprintln!("fire_pov: frames read={n}, emitted={emitted}, threshold={threshold}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_is_inclusive() {
        assert_eq!(accepted_row(31, 0.5, 0.5), Some(31));
        assert_eq!(accepted_row(4, 0.499, 0.5), None);
    }

    #[test]
    fn invalid_scores_never_emit_a_row() {
        for evidence in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(accepted_row(4, evidence, 0.5), None);
        }
        assert_eq!(accepted_row(32, 0.8, 0.5), None);
        assert_eq!(accepted_row(4, 0.8, f64::NAN), None);
    }

    #[test]
    fn abstention_does_not_change_texture_candidate() {
        assert_eq!(accepted_row(0, -1.0, 0.5), None);
        assert_eq!(accepted_row(0, 1.0, 0.5), Some(0));
    }
}
