//! Validate the no-strip phase detector against real training data (the mod's per-frame ground truth).
//!
//! `cargo run --release --example validate_phase -- JOINED.jsonl VIDEO.mkv [--trials N] [--seed N] [--rate R] [--window W]`
//!
//! Only the *pairing* is used: video frame `n` -> the game's displayed rows (`f0`, `f1`) and its tick. The video
//! contributes nothing but its packet timestamps (ffprobe metadata, no pixels). The detector itself never sees
//! a tick: it gets `(frame, pts, sheet-1 row)` exactly as a hand-labelled FireCrackle session would. The ticks
//! are used here only to judge what really happened.
//!
//! 1. data facts and shared animation-index consistency;
//! 2. the detector on the real labels, each flag explained from the ground truth;
//! 3. random cuts of the timeline (frames removed, later timestamps pulled forward), how many are caught.

use std::collections::HashSet;
use std::path::Path;
use studio_core::config::IndexConfig;
use studio_core::import::{probe, Tools};
use studio_core::label::Label;
use studio_core::phase::{analyze, Config, FlagKind, FrameObs, Severity};

struct Row {
    n: u32,
    tick: u32,
    f0: u8,
    f1: u8,
    fire: bool,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next() % (hi - lo) as u64) as usize
    }
}

fn circ_signed(d: f64) -> f64 {
    (d + 16.0).rem_euclid(32.0) - 16.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(joined), Some(video)) = (args.next(), args.next()) else {
        eprintln!("usage: validate_phase JOINED.jsonl VIDEO [--trials N] [--seed N] [--rate R] [--window W]");
        std::process::exit(2);
    };
    let mut trials = 400usize;
    let mut seed = 1u64;
    let mut cfg = Config::default();
    while let Some(a) = args.next() {
        let v = args.next().unwrap_or_default();
        match a.as_str() {
            "--trials" => trials = v.parse().unwrap(),
            "--seed" => seed = v.parse().unwrap(),
            "--rate" => cfg.tick_rate = v.parse().unwrap(),
            "--window" => cfg.window_ticks = v.parse().unwrap(),
            other => panic!("unknown option {other}"),
        }
    }

    // ---- load the pairing and the timestamps
    let text = std::fs::read_to_string(&joined).expect("read joined");
    let mut rows: Vec<Row> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).expect("json row");
        let label = &v["label"];
        if label.is_null() {
            continue;
        }
        rows.push(Row {
            n: v["n"].as_u64().unwrap() as u32,
            tick: label["tick"].as_u64().unwrap() as u32,
            f0: ((label["f0"].as_i64().unwrap().max(0) + 16) % 32) as u8,
            f1: label["f1"].as_i64().unwrap().max(0) as u8,
            fire: label["fire"].as_bool().unwrap_or(false),
        });
    }
    let (_, index) = probe(
        &Tools::default(),
        Path::new(&video),
        &IndexConfig::default(),
    )
    .expect("probe");
    assert!(
        rows.iter().all(|r| (r.n as usize) < index.frames.len()),
        "joined rows beyond the video's frames"
    );
    let pts = |n: u32| index.frames[n as usize].pts;

    // ---- 1. facts
    println!("== 1. data ==");
    let fire_rows: Vec<&Row> = rows.iter().filter(|r| r.fire).collect();
    let e2 = rows.iter().filter(|r| r.f0 == r.f1).count();
    println!(
        "frames with a label: {}   fire on screen: {}   video frames: {}",
        rows.len(),
        fire_rows.len(),
        index.frames.len()
    );
    println!(
        "Shared animation index: holds on {} of {} frames",
        e2,
        rows.len()
    );
    let dur = pts(rows[rows.len() - 1].n) - pts(rows[0].n);
    let dticks = rows[rows.len() - 1].tick as f64 - rows[0].tick as f64;
    println!(
        "duration {:.1} s, game ticks {}, average {:.3} ticks per video second (nominal 20)",
        dur,
        dticks,
        dticks / dur
    );

    // ground truth: the sprite follows executed ticks; its phase (tick - row) changes only on a reset
    let gt_phase = |r: &Row| (r.tick as i64 - r.f1 as i64).rem_euclid(32);
    let mut gt_resets: Vec<u32> = Vec::new();
    for w in rows.windows(2) {
        if gt_phase(&w[0]) != gt_phase(&w[1]) {
            gt_resets.push(w[1].n);
        }
    }
    println!(
        "ground-truth sprite resets (phase tick-row changes): {} at frames {:?}",
        gt_resets.len(),
        &gt_resets[..gt_resets.len().min(12)]
    );

    // ---- 2. the detector on the real labels
    println!("\n== 2. detector on the real labels (animation index of every fire frame; config {cfg:?}) ==");
    let base: Vec<FrameObs> = fire_rows
        .iter()
        .map(|r| FrameObs {
            frame: r.n,
            pts: pts(r.n),
            label: Label::Fire {
                sheet: 1,
                row: r.f1,
                mirrored: false,
            },
        })
        .collect();
    let a = analyze(&base, &cfg).expect("analyze");
    let hard = a
        .flags
        .iter()
        .filter(|f| f.severity == Severity::Hard)
        .count();
    println!(
        "runs {}   flags {} (hard {}, soft {})",
        a.runs.len(),
        a.flags.len(),
        hard,
        a.flags.len() - hard
    );
    let count = |k: FlagKind| a.flags.iter().filter(|f| f.kind == k).count();
    println!(
        "by kind: {} excursions (lag stalls that came back), {} shifts (changes that stayed), {} outlier frames",
        count(FlagKind::PhaseExcursion),
        count(FlagKind::PhaseShift),
        count(FlagKind::PhaseOutlier)
    );
    let widest = a.runs.iter().map(|r| r.width).fold(0.0, f64::max);
    println!(
        "widest run arc {:.2} ticks (window {:.1}); runs with >=100 frames: {}",
        widest,
        cfg.window_ticks,
        a.runs.iter().filter(|r| r.points >= 100).count()
    );

    let by_n: std::collections::HashMap<u32, usize> =
        rows.iter().enumerate().map(|(i, r)| (r.n, i)).collect();
    let mut explained = [0usize; 3]; // reset, lag burst, unexplained
    println!("\nfirst flags, each checked against the ground truth ticks:");
    for (k, f) in a.flags.iter().enumerate() {
        let i = by_n[&f.frame];
        let lo = i.saturating_sub(90);
        let hi = (i + 3).min(rows.len() - 1);
        let reset = rows[lo..hi]
            .windows(2)
            .any(|w| gt_phase(&w[0]) != gt_phase(&w[1]));
        let burst = rows[lo..=hi]
            .windows(2)
            .map(|w| w[1].tick as i64 - w[0].tick as i64)
            .max()
            .unwrap_or(0);
        let why = if reset {
            explained[0] += 1;
            "sprite reset in the ground truth".to_string()
        } else if burst >= 2 {
            explained[1] += 1;
            format!("lag: ticks advanced {burst} in one frame nearby")
        } else {
            explained[2] += 1;
            "unexplained".to_string()
        };
        if k < 25 {
            println!(
                "  frame {:6} t={:8.2}s {:?} {:?} delta {:+6.2}  <- {}",
                f.frame,
                pts(f.frame),
                f.kind,
                f.severity,
                f.phase_delta,
                why
            );
        }
    }
    println!(
        "all {} flags: {} next to a ground-truth reset, {} next to a lag burst, {} unexplained",
        a.flags.len(),
        explained[0],
        explained[1],
        explained[2]
    );
    let baseline_frames: HashSet<u32> = a.flags.iter().map(|f| f.frame).collect();

    // ---- 3. random cuts
    println!("\n== 3. random cuts ({trials} single cuts, seed {seed}) ==");
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    // bucket by |phase shift| of the cut: shift = -rate * removed seconds, mod 32
    let edges = [0.0, 1.0, 2.0, 4.0, 8.0, 16.01];
    let mut tried = [0usize; 5];
    let mut caught = [0usize; 5];
    for _ in 0..trials {
        let i = rng.range(60, base.len() - 60);
        let seconds = match rng.range(0, 3) {
            0 => 0.2 + rng.unit() * 2.0,   // a trimmed breath
            1 => 2.0 + rng.unit() * 20.0,  // a typical cut
            _ => 20.0 + rng.unit() * 90.0, // a long edit
        };
        // remove everything from base[i].pts up to base[i].pts + seconds; later frames move earlier by the gap
        let start = base[i].pts;
        let end = start + seconds;
        let j = base.iter().position(|o| o.pts >= end).unwrap_or(base.len());
        if j >= base.len() - 5 {
            continue;
        }
        let removed = base[j].pts - start; // exact video time that disappears (cut spans the gap to the next kept frame)
        let mut cut: Vec<FrameObs> = base[..i].to_vec();
        cut.extend(base[j..].iter().map(|o| FrameObs {
            pts: o.pts - removed,
            ..*o
        }));
        let shift = circ_signed(-cfg.tick_rate * removed).abs();
        let b = (0..5)
            .find(|&b| shift >= edges[b] && shift < edges[b + 1])
            .unwrap();
        tried[b] += 1;
        let c = analyze(&cut, &cfg).expect("analyze cut");
        // detected: a flag that the clean data did not have, on the first frames after the cut point
        let first_after = base[j].frame;
        let near: Vec<u32> = base[j..].iter().take(4).map(|o| o.frame).collect();
        // an excursion whose return point is the cut also counts: a cut that lands back on an earlier level
        // looks like a stall that ends at the cut, and its last frame is the last kept frame before the cut
        let before: Vec<u32> = base[i.saturating_sub(3)..i]
            .iter()
            .map(|o| o.frame)
            .collect();
        if c.flags
            .iter()
            .any(|f| near.contains(&f.frame) && !baseline_frames.contains(&f.frame))
            || c.flags
                .iter()
                .any(|f| f.frame == first_after && f.kind == FlagKind::PhaseShift)
            || c.flags.iter().any(|f| {
                f.kind == FlagKind::PhaseExcursion
                    && before.contains(&f.end_frame)
                    && !a
                        .flags
                        .iter()
                        .any(|g| g.frame == f.frame && g.end_frame == f.end_frame)
            })
        {
            caught[b] += 1;
        } else if shift >= 4.0 && std::env::var("SHOW_MISSES").is_ok() {
            let nearby: Vec<String> = c
                .flags
                .iter()
                .filter(|f| f.frame + 400 >= first_after && f.frame <= first_after + 400)
                .map(|f| {
                    format!(
                        "{:?}@{}..{} {:+.1}",
                        f.kind, f.frame, f.end_frame, f.phase_delta
                    )
                })
                .collect();
            let baseline_near: Vec<String> = a
                .flags
                .iter()
                .filter(|f| f.frame + 400 >= first_after && f.frame <= first_after + 400)
                .map(|f| {
                    format!(
                        "{:?}@{}..{} {:+.1}",
                        f.kind, f.frame, f.end_frame, f.phase_delta
                    )
                })
                .collect();
            println!(
                "MISS cut before frame {first_after} (removed {removed:.2}s, shift {shift:.1}); flags nearby after cut {nearby:?}; baseline nearby {baseline_near:?}"
            );
            if std::env::var("SHOW_ALL_FLAGS").is_ok() {
                for f in &c.flags {
                    println!(
                        "       flag {:?}@{}..{} {:?} {:+.2}",
                        f.kind, f.frame, f.end_frame, f.severity, f.phase_delta
                    );
                }
            }
            let at = c
                .runs
                .iter()
                .position(|r| r.first_frame >= first_after)
                .unwrap_or(c.runs.len());
            println!(
                "     (runs {}..{} of {}; flags overall {})",
                at.saturating_sub(3),
                (at + 2).min(c.runs.len()),
                c.runs.len(),
                c.flags.len()
            );
            for r in c.runs[at.saturating_sub(3)..(at + 2).min(c.runs.len())].iter() {
                println!(
                    "     run frames {:6}..{:6} n={:4} center {:6.2} width {:.2}",
                    r.first_frame, r.last_frame, r.points, r.center, r.width
                );
            }
        }
    }
    println!("cut size as phase shift |delta| (ticks mod 32)   cuts   caught   rate");
    let mut all = (0usize, 0usize);
    for b in 0..5 {
        if tried[b] == 0 {
            continue;
        }
        println!(
            "  {:>5.1} .. {:>5.1}                                {:5}  {:6}   {:5.1}%",
            edges[b],
            edges[b + 1].min(16.0),
            tried[b],
            caught[b],
            100.0 * caught[b] as f64 / tried[b] as f64
        );
        all.0 += tried[b];
        all.1 += caught[b];
    }
    println!(
        "  overall                                          {:5}  {:6}   {:5.1}%",
        all.0,
        all.1,
        100.0 * all.1 as f64 / all.0.max(1) as f64
    );
    println!("(a cut whose removed time is within ~1 tick of a multiple of 1.6 s is invisible to any phase check)");
}
