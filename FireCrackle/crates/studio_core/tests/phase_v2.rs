use studio_core::{
    config::Settings,
    label::Label,
    phase::{self, FrameObs, Severity},
    phase_v2::{analyze, Explanation},
};
fn clock(n: u32) -> Vec<FrameObs> {
    (0..n)
        .map(|i| FrameObs {
            frame: i,
            pts: i as f64 / 60.0,
            label: Label::Fire {
                sheet: 1,
                row: ((i / 3 + 7) % 32) as u8,
                mirrored: false,
            },
        })
        .collect()
}
fn shift(obs: &mut [FrameObs], by: u8) {
    for o in obs {
        if let Label::Fire {
            sheet,
            row,
            mirrored,
        } = o.label
        {
            o.label = Label::Fire {
                sheet,
                row: (row + by) % 32,
                mirrored,
            };
        }
    }
}
#[test]
fn clean_empty_and_single_observations_are_quiet() {
    for obs in [vec![], clock(1), clock(900)] {
        let a = analyze(&obs, &Settings::default()).unwrap();
        assert!(a.flags.is_empty());
        assert_eq!(a.points.len(), obs.len());
    }
}
#[test]
fn legacy_evidence_is_never_discarded_even_inside_a_recovered_stall() {
    let mut obs = clock(900);
    let held = obs[299].label;
    for o in &mut obs[300..444] {
        o.label = held;
    }
    shift(&mut obs[350..351], 12);
    let settings = Settings::default();
    let old = phase::analyze(&obs, &settings.phase).unwrap();
    let new = analyze(&obs, &settings).unwrap();
    for f in old.flags {
        assert!(
            new.flags
                .iter()
                .any(|g| g.frame == f.frame && g.end_frame == f.end_frame && g.kind == f.kind),
            "lost {f:?}"
        );
    }
    assert!(new.flags.iter().any(|f| f.frame == 350));
}
#[test]
fn single_wrong_row_and_wrong_sheet_survive_binning() {
    for amount in [5, 16] {
        let mut obs = clock(900);
        shift(&mut obs[173..174], amount);
        let a = analyze(&obs, &Settings::default()).unwrap();
        assert!(a.flags.iter().any(|f| f.frame == 173));
    }
}
#[test]
fn sustained_change_is_reported_once_with_an_explanation() {
    let mut obs = clock(900);
    shift(&mut obs[173..], 5);
    let a = analyze(&obs, &Settings::default()).unwrap();
    assert_eq!(
        a.flags
            .iter()
            .filter(|f| f.frame.abs_diff(173) <= 3)
            .count(),
        1
    );
    assert!(!a.flag_details.is_empty());
    assert!(a.reference_phase.is_some());
}
#[test]
fn extrapolates_through_a_hold_longer_than_a_cycle_and_checks_recovery() {
    let mut obs = clock(900);
    let held = obs[299].label;
    for o in &mut obs[300..444] {
        o.label = held;
    }
    let a = analyze(&obs, &Settings::default()).unwrap();
    let h = a
        .lag_intervals
        .iter()
        .find(|h| h.first_frame <= 300 && h.last_frame >= 443)
        .unwrap();
    assert_eq!(h.recovered, Some(true));
    assert!(h.elapsed_ticks > 47.0);
    assert!(a
        .flag_details
        .iter()
        .any(|d| d.explanation == Explanation::RecoveredHold));
    assert!(a.flags.iter().any(|f| f.severity == Severity::Hard)); // hard may be an actual lag discrepancy
    let end = analyze(&obs[..444], &Settings::default()).unwrap();
    assert_eq!(end.lag_intervals.last().unwrap().recovered, None);
}
#[test]
fn a_hold_that_does_not_recover_the_clock_stays_a_mismatch() {
    let mut obs = clock(900);
    let held = obs[299].label;
    for o in &mut obs[300..444] {
        o.label = held;
    }
    shift(&mut obs[444..], 5);
    let a = analyze(&obs, &Settings::default()).unwrap();
    assert!(a.lag_intervals.iter().any(|h| h.recovered == Some(false)));
    assert!(a
        .flag_details
        .iter()
        .any(|d| d.explanation == Explanation::HoldWithMismatch));
}
#[test]
fn long_gaps_are_checked_but_never_called_a_confirmed_hold() {
    let original = clock(900);
    let mut obs = original[..120].to_vec();
    obs.extend(original[600..].iter().map(|o| FrameObs {
        frame: o.frame + 19200,
        pts: o.pts + 320.0,
        ..*o
    }));
    let a = analyze(&obs, &Settings::default()).unwrap();
    assert!(a.lag_intervals.is_empty());
    assert!(a.flags.is_empty());
    let reference = a.reference_phase;
    shift(&mut obs[120..], 5);
    let b = analyze(&obs, &Settings::default()).unwrap();
    assert_eq!(reference, b.reference_phase);
    assert!(b.flags.iter().any(|f| f.frame >= 19800));
}
#[test]
fn settings_are_exposed_saved_and_change_the_result() {
    let mut s = Settings::default();
    for key in [
        "phase_v2.bin_seconds",
        "phase_v2.penalty_ticks2",
        "phase_v2.anchor_bins",
        "phase_v2.held_seconds",
        "phase_v2.held_max_gap_seconds",
        "phase_v2.recovery_samples",
        "phase_v2.recovery_ticks",
        "phase_v2.supplement_min_ticks",
    ] {
        assert!(s.get(key).is_some(), "{key}");
    }
    s.set("phase_v2.penalty_ticks2", 3.0).unwrap();
    let round: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(round, s);
    let mut obs = clock(900);
    shift(&mut obs[173..], 5);
    s.set("phase.min_shift_ticks", 8.0).unwrap();
    assert!(analyze(&obs, &s).unwrap().flags.is_empty());
    s.phase_v2.bin_seconds = 0.0;
    assert!(analyze(&obs, &s).is_err());
}

#[test]
fn sparse_gap_noise_inside_the_allowed_arc_does_not_add_a_supplement() {
    let obs: Vec<FrameObs> = (0..60)
        .map(|i| FrameObs {
            frame: i + if i >= 30 { 19200 } else { 0 },
            pts: 1.6 * i as f64 + if i >= 30 { 320.0 + 1.2 / 20.0 } else { 0.0 },
            label: Label::Fire {
                sheet: 1,
                row: 0,
                mirrored: false,
            },
        })
        .collect();
    assert!(phase::analyze(&obs, &Settings::default().phase)
        .unwrap()
        .flags
        .is_empty());
    assert!(analyze(&obs, &Settings::default())
        .unwrap()
        .flags
        .is_empty());
}

#[test]
fn a_short_wrong_label_does_not_break_recovered_hold_evidence() {
    let mut obs = clock(900);
    let held = obs[299].label;
    for o in &mut obs[300..444] {
        o.label = held;
    }
    shift(&mut obs[350..351], 12);
    let a = analyze(&obs, &Settings::default()).unwrap();
    assert!(a
        .lag_intervals
        .iter()
        .any(|h| h.first_frame <= 300 && h.last_frame >= 443 && h.recovered == Some(true)));
    assert!(a
        .flag_details
        .iter()
        .any(|d| d.frame == 350 && d.explanation == Explanation::LocalDeparture));
}
