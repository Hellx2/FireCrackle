//! `cargo run --release --example thumbs_smoke -- VIDEO`
//! Opens a video as a workspace, makes its timeline thumbnails (one keyframe pass), prints the timings and what was
//! stored. Use a copy of the video: a `.firecrackle` folder is created next to it.

use std::path::Path;
use std::time::Instant;
use studio_core::config::Settings;
use studio_core::import::Tools;
use studio_core::workspace::Workspace;

fn main() {
    let Some(video) = std::env::args().nth(1) else {
        eprintln!("usage: thumbs_smoke VIDEO");
        std::process::exit(2);
    };
    let mut w =
        Workspace::open(&Tools::default(), Path::new(&video), &Settings::default()).expect("open");
    let v = w.view();
    println!("{} frames at {:.2} fps", v.frame_count, v.fps);
    println!("before: {:?}", w.thumbs_status());
    let t = Instant::now();
    w.thumbs_start();
    w.wait_thumbs();
    let s = w.thumbs_status();
    println!("pass: {:.2}s -> {:?}", t.elapsed().as_secs_f64(), s);
    let wanted: Vec<u32> = (0..5).map(|i| i * v.frame_count / 5).collect();
    let near = w.thumb_nearest(&wanted);
    for (f, n) in wanted.iter().zip(&near) {
        match n.map(|n| w.thumb_jpeg(n)) {
            Some(Ok(b)) => println!(
                "frame {f:6}: thumbnail of frame {:6} ({} bytes)",
                n.unwrap(),
                b.len()
            ),
            other => println!("frame {f:6}: {other:?}"),
        }
    }
    let t = Instant::now();
    w.thumbs_start();
    println!(
        "second start (cached): {:?} in {:.1} ms",
        w.thumbs_status().state,
        t.elapsed().as_secs_f64() * 1000.0
    );
}
