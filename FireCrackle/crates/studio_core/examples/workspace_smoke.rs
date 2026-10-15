//! `cargo run --release --example workspace_smoke -- VIDEO`
//! Opens a video as a workspace (a `.firecrackle` folder is created next to it), decodes frames from several
//! places, labels a few and prints timings. Use a copy of the video: this writes next to it.

use std::path::Path;
use std::time::Instant;
use studio_core::config::Settings;
use studio_core::import::Tools;
use studio_core::workspace::{UiAction, Workspace};

fn main() {
    let Some(video) = std::env::args().nth(1) else {
        eprintln!("usage: workspace_smoke VIDEO");
        std::process::exit(2);
    };
    let t = Instant::now();
    let mut w = Workspace::open(&Tools::default(), Path::new(&video), &Settings::default()).expect("open");
    let v = w.view();
    println!("opened in {:.2}s: {} frames, {:.2} fps, warnings: {:?}", t.elapsed().as_secs_f64(), v.frame_count, v.fps, v.warnings);

    let last = v.frame_count - 1;
    for f in [0, 1, 2, 1000, 1001, 4000, 4001, 4239, 4240, 5000, last / 2, last - 1, last, 3, 100] {
        let t = Instant::now();
        match w.frame_jpeg(f) {
            Ok(b) => println!("frame {f:6}: {:7} bytes in {:7.1} ms", b.len(), t.elapsed().as_secs_f64() * 1000.0),
            Err(e) => println!("frame {f:6}: ERROR {e:?}"),
        }
    }
    w.wait_idle();

    let t = Instant::now();
    let mut ok = 0;
    for f in 5000..5200 {
        if w.frame_jpeg(f).is_ok() {
            ok += 1;
        }
    }
    println!("200 sequential frames: {ok} ok in {:.1} ms ({:.2} ms each, with prefetch)", t.elapsed().as_secs_f64() * 1000.0, t.elapsed().as_secs_f64() * 5.0);
    w.wait_idle();

    let t = Instant::now();
    w.apply(UiAction::GoTo { frame: 5000 }).unwrap();
    for row in 3..13u32 {
        w.apply(UiAction::ClickCell { col: 1, row }).unwrap();
    }
    let v = w.view();
    println!("10 labels applied in {:.1} ms; labelled {}, current {}, suggestion {:?}", t.elapsed().as_secs_f64() * 1000.0, v.labelled_count, v.current, v.suggestion);
}
