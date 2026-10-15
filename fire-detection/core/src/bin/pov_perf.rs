//! Reproducible full-resolution POV scan benchmark. CSV output includes exact result hashes.
use fire_core::{
    overlay::SilhouetteMatcher,
    veil::detect_veil,
    video::{parallel_frames, parse_probe},
};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
struct TimedReader<R> {
    inner: R,
    nanos: AtomicU64,
}
impl<R: Read> Read for &TimedReader<std::sync::Mutex<R>> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let t = Instant::now();
        let result = self.inner.lock().unwrap().read(buffer);
        self.nanos
            .fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
        result
    }
}
fn main() {
    run().unwrap();
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let video = args.get(1).ok_or("pov_perf VIDEO OUTCSV [LIMIT]")?;
    let output = args.get(2).ok_or("OUTCSV required")?;
    let limit = args
        .get(3)
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(96);
    let workers = args
        .get(4)
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(1);
    let ffmpeg = std::env::var("POV_FFMPEG").unwrap_or("ffmpeg".into());
    let ffprobe = std::env::var("POV_FFPROBE").unwrap_or("ffprobe".into());
    let probe = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-of",
            "default=nw=1",
        ])
        .arg(video)
        .output()
        .map_err(|e| e.to_string())?;
    let info = parse_probe(&String::from_utf8_lossy(&probe.stdout))?;
    let (w, h) = (info.width, info.height);
    let sheet = include_bytes!("../../../data/assets/fire_1.rgba");
    let a = SilhouetteMatcher::new(sheet, w, h, 70.)?;
    let b = SilhouetteMatcher::new(sheet, w, h, 60.)?;
    let mut decoder = Command::new(ffmpeg);
    decoder.args(["-nostdin", "-v", "error"]);
    if let Ok(hw) = std::env::var("POV_HWACCEL") {
        decoder.args(["-hwaccel", &hw]);
    }
    if let Ok(threads) = std::env::var("POV_THREADS") {
        decoder.args(["-threads", &threads]);
    }
    let mut child = decoder
        .arg("-i")
        .arg(video)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            &limit.to_string(),
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
        .map_err(|e| e.to_string())?;
    let pipe = TimedReader {
        inner: std::sync::Mutex::new(child.stdout.take().unwrap()),
        nanos: AtomicU64::new(0),
    };
    let mut out =
        std::io::BufWriter::new(std::fs::File::create(output).map_err(|e| e.to_string())?);
    writeln!(out, "n,veiled,row,fov,evidence,hash").unwrap();
    let wall = Instant::now();
    let veils = AtomicU64::new(0);
    let matches = AtomicU64::new(0);
    let mut results = Vec::new();
    parallel_frames(
        &pipe,
        w * h * 3,
        limit,
        workers,
        &AtomicBool::new(false),
        |frame| {
            let t = Instant::now();
            let veiled = detect_veil(frame, w, h, 16.min(h - 1))?;
            veils.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let t = Instant::now();
            let ra = if veiled {
                a.match_frame_veiled(frame)?
            } else {
                a.match_frame(frame)?
            };
            let rb = if veiled {
                b.match_frame_veiled(frame)?
            } else {
                b.match_frame(frame)?
            };
            matches.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let (best, fov) = if rb.colour_evidence > ra.colour_evidence {
                (&rb, 60)
            } else {
                (&ra, 70)
            };
            let mut hash = 0xcbf29ce484222325u64;
            for r in [&ra, &rb] {
                for (row, score) in r.ranked.iter().chain(r.ranked_colour.iter()) {
                    hash = (hash ^ (*row as u64)).wrapping_mul(0x100000001b3);
                    hash = (hash ^ score.to_bits()).wrapping_mul(0x100000001b3);
                }
                hash = (hash ^ r.presence.to_bits()).wrapping_mul(0x100000001b3);
                hash = (hash ^ r.colour_evidence.to_bits()).wrapping_mul(0x100000001b3);
            }
            Ok((
                veiled,
                best.ranked_colour[0].0,
                fov,
                best.colour_evidence,
                hash,
            ))
        },
        |n, value| results.push((n, value)),
    )?;
    let seconds = wall.elapsed().as_secs_f64();
    let _ = child.wait();
    results.sort_by_key(|v| v.0);
    for (n, (veiled, row, fov, evidence, hash)) in &results {
        writeln!(out, "{n},{veiled},{row},{fov},{evidence:.17},{hash:016x}").unwrap();
    }
    out.flush().unwrap();
    let n = results.len();
    eprintln!("frames={n} workers={workers} size={w}x{h} seconds={seconds:.3} fps={:.1} read_ms={:.1} veil_ms={:.1} match_ms={:.1}",n as f64/seconds,pipe.nanos.load(Ordering::Relaxed) as f64/1e6,veils.load(Ordering::Relaxed) as f64/1e6,matches.load(Ordering::Relaxed) as f64/1e6);
    Ok(())
}
