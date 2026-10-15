//! Real captures are noisy (training-data finding: 90% of frames within 0.7 tick of their local phase level,
//! 1% beyond 2 ticks, 44 lag stalls in 9 minutes). Two rules keep the flag list reviewable:
//! a level that leaves and comes back is ONE excursion (lag); changes under the noise floor are not flagged.

use studio_core::label::Label;
use studio_core::phase::*;

// ---------------------------------------------------------------- helpers

fn row_of(s: i64) -> u8 {
    s.rem_euclid(32) as u8 // sheet 1: row == step (E2)
}

fn fire(frame: u32, pts: f64, s: i64) -> FrameObs {
    FrameObs { frame, pts, label: Label::Fire { sheet: 1, row: row_of(s), mirrored: false } }
}

fn run(o: &[FrameObs]) -> Analysis {
    analyze(o, &Config::default()).unwrap()
}

fn kinds(a: &Analysis) -> Vec<(u32, FlagKind, Severity)> {
    a.flags.iter().map(|f| (f.frame, f.kind, f.severity)).collect()
}

/// fps 20 = one tick per frame: `lens[j]` frames, each labelled `extra[j]` ticks ahead of the true step,
/// so the phase of run `j` is the constant `-3 - extra[j]`.
fn runs_of(lens: &[u32], extra: &[i64]) -> Vec<FrameObs> {
    let mut o = Vec::new();
    let mut f = 0u32;
    for (&n, &e) in lens.iter().zip(extra) {
        for _ in 0..n {
            o.push(fire(f, f as f64 / 20.0, 3 + f as i64 + e));
            f += 1;
        }
    }
    o
}

/// Frames whose estimated phase is exactly `phis[i]` (at 20 ticks/s): frame `i` shows step `4i mod 32` and its
/// timestamp is placed so that `20 * pts - step = phis[i]`. Needs `phis[i+1] - phis[i] > -4`.
fn from_phases(phis: &[f64]) -> Vec<FrameObs> {
    phis.iter()
        .enumerate()
        .map(|(i, &p)| fire(i as u32, (4.0 * i as f64 + p) / 20.0, 4 * i as i64))
        .collect()
}

// ------------------------------------------------------------ excursions

#[test]
fn a_level_that_comes_back_is_one_excursion_not_two_shifts() {
    let a = run(&runs_of(&[10, 5, 10], &[0, 8, 0]));
    assert_eq!(kinds(&a), vec![(10, FlagKind::PhaseExcursion, Severity::Soft)]);
    assert_eq!(a.flags[0].end_frame, 14);
    assert!((a.flags[0].phase_delta + 8.0).abs() < 1.0, "delta {}", a.flags[0].phase_delta);
}

#[test]
fn an_excursion_further_than_max_lag_is_hard() {
    let a = run(&runs_of(&[10, 5, 10], &[0, 12, 0]));
    assert_eq!(kinds(&a), vec![(10, FlagKind::PhaseExcursion, Severity::Hard)]);
}

#[test]
fn a_staircase_back_is_one_excursion_with_the_peak_delta() {
    // away by 4, then by 9, then home: one event
    let a = run(&runs_of(&[10, 4, 4, 10], &[0, 4, 9, 0]));
    assert_eq!(kinds(&a), vec![(10, FlagKind::PhaseExcursion, Severity::Soft)]);
    assert_eq!(a.flags[0].end_frame, 17);
    assert!((a.flags[0].phase_delta + 9.0).abs() < 1.0, "peak delta {}", a.flags[0].phase_delta);
}

#[test]
fn a_change_that_stays_is_still_a_shift() {
    let a = run(&runs_of(&[10, 10], &[0, 8]));
    assert_eq!(kinds(&a), vec![(10, FlagKind::PhaseShift, Severity::Soft)]);
    assert_eq!(a.flags[0].end_frame, 10);
}

#[test]
fn a_return_after_max_excursion_seconds_is_two_shifts_and_the_limit_is_configurable() {
    // 700 frames at 20 fps = 35 s away from the level
    let o = runs_of(&[10, 700, 10], &[0, 8, 0]);
    let a = run(&o);
    assert_eq!(kinds(&a), vec![(10, FlagKind::PhaseShift, Severity::Soft), (710, FlagKind::PhaseShift, Severity::Soft)]);
    let lax = analyze(&o, &Config { max_excursion_seconds: 60.0, ..Config::default() }).unwrap();
    assert_eq!(kinds(&lax), vec![(10, FlagKind::PhaseExcursion, Severity::Soft)]);
    let strict = analyze(&runs_of(&[10, 5, 10], &[0, 8, 0]), &Config { max_excursion_seconds: 0.1, ..Config::default() }).unwrap();
    assert!(strict.flags.iter().all(|f| f.kind == FlagKind::PhaseShift), "5 frames = 0.25 s is beyond 0.1 s");
}

#[test]
fn a_return_to_a_different_level_is_not_an_excursion() {
    let a = run(&runs_of(&[10, 5, 10], &[0, 8, 16]));
    assert_eq!(a.flags.len(), 2);
    assert!(a.flags.iter().all(|f| f.kind == FlagKind::PhaseShift));
}

#[test]
fn several_separate_stalls_give_one_flag_each() {
    let o = runs_of(&[20, 4, 20, 5, 20, 6, 20], &[0, 7, 0, 9, 0, 6, 0]);
    let a = run(&o);
    assert_eq!(a.flags.len(), 3, "{:?}", kinds(&a));
    assert!(a.flags.iter().all(|f| f.kind == FlagKind::PhaseExcursion));
    let starts: Vec<u32> = a.flags.iter().map(|f| f.frame).collect();
    assert_eq!(starts, vec![20, 44, 69]);
}

// ------------------------------------------------------------- the cut case

#[test]
fn a_cut_in_the_footage_is_a_shift_with_the_right_size() {
    // 60 fps footage of a steady game; frames 200..349 are cut out (150 frames = 2.5 s = 50 ticks) and the
    // later timestamps move 2.5 s earlier. The sprite did not move: the phase jumps by -50 = +14 mod 32.
    let g_offset = 0.37;
    let step = |t: f64| (9 + (20.0 * (t + g_offset)).floor() as i64).rem_euclid(32);
    let mut o: Vec<FrameObs> = Vec::new();
    for i in 0..600u32 {
        let t = i as f64 / 60.0;
        if (200..350).contains(&i) {
            continue;
        }
        let pts = if i >= 350 { t - 150.0 / 60.0 } else { t };
        o.push(fire(i, pts, step(t)));
    }
    let a = run(&o);
    assert_eq!(kinds(&a), vec![(350, FlagKind::PhaseShift, Severity::Soft)]);
    assert!((a.flags[0].phase_delta - 14.0).abs() < 1.0, "delta {}", a.flags[0].phase_delta);
}

// ------------------------------------------------------------ min shift

#[test]
fn a_change_under_min_shift_is_noise_and_not_flagged() {
    // 30 frames whose phase alternates 0.0 / 2.0 (a 2-tick arc), then 30 frames alternating 0.6 / 2.6: the arcs
    // do not fit one window together, but their centres differ by only 0.6 tick: timing noise, not an event
    let mut phis: Vec<f64> = Vec::new();
    phis.extend((0..30).map(|i| if i % 2 == 0 { 0.0 } else { 2.0 }));
    phis.extend((0..30).map(|i| if i % 2 == 0 { 0.6 } else { 2.6 }));
    let o = from_phases(&phis);
    assert!(run(&o).flags.is_empty(), "{:?}", run(&o).flags);
    let sensitive = analyze(&o, &Config { min_shift_ticks: 0.5, ..Config::default() }).unwrap();
    assert_eq!(sensitive.flags.len(), 1, "{:?}", sensitive.flags);
    let f = sensitive.flags[0];
    assert_eq!(f.kind, FlagKind::PhaseShift);
    assert!((30..=32).contains(&f.frame), "frame {}", f.frame);
    assert!((f.phase_delta - 0.6).abs() < 0.2, "delta {}", f.phase_delta);
}

#[test]
fn min_shift_zero_flags_every_boundary() {
    let mut phis: Vec<f64> = Vec::new();
    phis.extend((0..30).map(|i| if i % 2 == 0 { 0.0 } else { 2.0 }));
    phis.extend((0..30).map(|i| if i % 2 == 0 { 0.6 } else { 2.6 }));
    let o = from_phases(&phis);
    let all = analyze(&o, &Config { min_shift_ticks: 0.0, ..Config::default() }).unwrap();
    assert!(!all.flags.is_empty());
}

#[test]
fn min_shift_also_filters_outliers_and_excursions() {
    // a single frame 5 ticks off is flagged by default and ignored when min_shift is above 5
    let mut o = runs_of(&[10, 1, 10], &[0, 5, 0]);
    assert_eq!(run(&o).flags.len(), 1);
    let cfg = Config { min_shift_ticks: 6.0, ..Config::default() };
    assert!(analyze(&o, &cfg).unwrap().flags.is_empty());
    // same for a 5-frame excursion
    o = runs_of(&[10, 5, 10], &[0, 5, 0]);
    assert_eq!(run(&o).flags.len(), 1);
    assert!(analyze(&o, &cfg).unwrap().flags.is_empty());
}

// --------------------------------------------------------- flag extents

#[test]
fn outlier_flags_cover_exactly_their_own_frame() {
    let a = run(&runs_of(&[10, 2, 10], &[0, 8, 0]));
    assert_eq!(a.flags.len(), 2);
    for f in &a.flags {
        assert_eq!(f.kind, FlagKind::PhaseOutlier);
        assert_eq!(f.end_frame, f.frame);
    }
}

#[test]
fn flags_stay_sorted_by_frame_with_excursions_and_shifts_mixed() {
    // an excursion at 10, then a permanent change at 40
    let a = run(&runs_of(&[10, 5, 25, 20], &[0, 8, 0, 9]));
    let frames: Vec<(u32, FlagKind)> = a.flags.iter().map(|f| (f.frame, f.kind)).collect();
    assert_eq!(frames, vec![(10, FlagKind::PhaseExcursion), (40, FlagKind::PhaseShift)]);
}
