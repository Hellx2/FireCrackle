//! `cargo run --release --example import_smoke -- VIDEO [project.json]`
//! Imports a video with default settings and prints the summary (and optionally saves the project).

use std::path::Path;
use std::time::Instant;
use studio_core::config::Settings;
use studio_core::import::{import_video, Tools};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(video) = args.next() else {
        eprintln!("usage: import_smoke VIDEO [project.json]");
        std::process::exit(2);
    };
    let started = Instant::now();
    match import_video(&Tools::default(), Path::new(&video), &Settings::default()) {
        Ok(p) => {
            println!(
                "video    : {} ({} bytes, fingerprint {})",
                p.video_path, p.video_size, p.fingerprint
            );
            println!(
                "meta     : {}x{} {}/{} fps {}",
                p.meta.width, p.meta.height, p.meta.fps_num, p.meta.fps_den, p.meta.codec
            );
            println!("index    : {:?}", p.index);
            println!(
                "warnings : {}",
                if p.warnings.is_empty() { "none" } else { "" }
            );
            for w in &p.warnings {
                println!("  - {w}");
            }
            println!(
                "imported {} frames in {:.2}s",
                p.frames.len(),
                started.elapsed().as_secs_f64()
            );
            if let Some(out) = args.next() {
                p.save(Path::new(&out)).expect("save");
                println!("saved {out}");
            }
        }
        Err(e) => {
            eprintln!("import failed: {e:?}");
            std::process::exit(1);
        }
    }
}
