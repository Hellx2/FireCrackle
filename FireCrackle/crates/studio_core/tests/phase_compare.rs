use studio_core::config::PhaseResearchConfig;
use studio_core::label::Label;
use studio_core::phase::FrameObs;
use studio_core::phase_compare::*;

fn phases(xs: &[f64]) -> Vec<FrameObs> {
    xs.iter().enumerate().map(|(i, &p)| FrameObs {
        frame: i as u32,
        pts: (32.0 * i as f64 + p) / 20.0,
        label: Label::Fire { sheet: 1, row: 0, mirrored: false },
    }).collect()
}
fn clock(n: u32) -> Vec<FrameObs> {
    (0..n).map(|i| FrameObs { frame: i, pts: i as f64 / 60.0,
        label: Label::Fire { sheet: 1, row: ((i / 3 + 7) % 32) as u8, mirrored: false } }).collect()
}
fn oracle(xs: &[f64], penalty: f64) -> f64 {
    if xs.is_empty() { return 0.0; }
    (0..1usize << (xs.len()-1)).map(|mask| {
        let mut total = 0.0;
        let mut lo = 0;
        for hi in 1..=xs.len() {
            if hi == xs.len() || mask & (1 << (hi-1)) != 0 {
                let angles: Vec<f64> = xs[lo..hi].iter().map(|x| x * std::f64::consts::TAU / 32.0).collect();
                let c: f64 = angles.iter().map(|x| x.cos()).sum();
                let s: f64 = angles.iter().map(|x| x.sin()).sum();
                total += 2.0 * (32.0/std::f64::consts::TAU).powi(2) * ((hi-lo) as f64 - c.hypot(s)).max(0.0);
                if lo > 0 { total += penalty; }
                lo = hi;
            }
        }
        total
    }).fold(f64::INFINITY, f64::min)
}

#[test]
fn pelt_matches_exhaustive_partition_oracle_including_wrap_and_opposite_jumps() {
    let mut rng=17u64;
    for n in 1..=9 {
        for _ in 0..30 {
            let xs: Vec<f64>=(0..n).map(|_| { rng=rng.wrapping_mul(6364136223846793005).wrapping_add(1); (rng>>32) as f64 / u32::MAX as f64 *32.0 }).collect();
            for penalty in [0.0,0.1,3.0,100.0] {
                let result=pelt(&xs,penalty).unwrap();
                assert!((result.objective-oracle(&xs,penalty)).abs()<1e-7,"{xs:?}, {penalty}: {result:?}");
            }
        }
    }
}
#[test]
fn simple_pelt_segments_and_wrap() {
    assert_eq!(pelt(&[],2.0).unwrap().starts,Vec::<usize>::new());
    assert_eq!(pelt(&[31.9,0.1,31.9,0.1],2.0).unwrap().starts,vec![0]);
    assert_eq!(pelt(&[0.0,0.0,8.0,8.0,0.0,0.0],2.0).unwrap().starts,vec![0,2,4]);
}
#[test]
fn all_methods_accept_clean_clock_and_empty() {
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        assert!(compare(&[],method,&PhaseResearchConfig::default()).unwrap().events.is_empty());
        assert!(compare(&clock(900),method,&PhaseResearchConfig::default()).unwrap().events.is_empty(),"{method:?}");
    }
}
#[test]
fn all_methods_find_a_persistent_change_and_a_return() {
    let xs: Vec<f64>=(0..90).map(|i| if (30..60).contains(&i) {8.0} else {0.0}).collect();
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        let a=compare(&phases(&xs),method,&PhaseResearchConfig::default()).unwrap();
        assert!(a.events.iter().any(|e| e.first_frame==30),"{method:?}: {:?}",a.events);
        assert!(a.events.iter().any(|e| e.last_frame>=59 && e.kind==EventKind::RecoveredDeparture && e.soft),"{method:?}: {:?}",a.events);
    }
}
#[test]
fn long_gap_is_checked_against_the_same_clock() {
    let mut xs=vec![0.0;20]; xs.extend(vec![5.0;20]);
    let mut obs=phases(&xs);
    for o in &mut obs[20..] { o.pts+=320.0; o.frame+=19200; }
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        let a=compare(&obs,method,&PhaseResearchConfig::default()).unwrap();
        assert!(a.events.iter().any(|e| e.first_frame==19220),"{method:?}");
        assert!((a.reference_phase-0.0).abs()<0.1 || (a.reference_phase-32.0).abs()<0.1);
    }
}
#[test]
fn matching_long_gap_and_whole_cycle_are_invisible() {
    let mut obs=clock(120);
    for o in &mut obs[60..] { o.pts+=320.0; o.frame+=19200; }
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        assert!(compare(&obs,method,&PhaseResearchConfig::default()).unwrap().events.is_empty());
    }
}
#[test]
fn recovered_freeze_over_a_cycle_is_soft_and_not_a_label_error() {
    let mut obs=clock(900);
    let held=obs[299].label;
    for o in &mut obs[300..444] { o.label=held; }
    let a=compare(&obs,Method::Fixed,&PhaseResearchConfig::default()).unwrap();
    assert!(a.events.iter().any(|e| e.kind==EventKind::HeldAnimation && e.first_frame<=300 && e.last_frame>=443 && e.soft));
}
#[test]
fn small_persistent_offset_can_be_detected_using_many_observations() {
    let xs:Vec<f64>=(0..200).map(|i| if i<100 {31.8} else {0.4}).collect();
    let cfg=PhaseResearchConfig { min_change_ticks:0.4, penalty_ticks2:2.0,..Default::default() };
    for method in [Method::Pelt,Method::Pcid] {
        let a=compare(&phases(&xs),method,&cfg).unwrap();
        assert!(a.events.iter().any(|e| e.first_frame==100),"{method:?}: {:?}",a.events);
    }
}
#[test]
fn bad_input_and_config_return_errors() {
    assert!(pelt(&[f64::NAN],1.0).is_err());
    assert!(pelt(&[0.0],-1.0).is_err());
    for cfg in [PhaseResearchConfig { bin_seconds:0.0,..Default::default() },PhaseResearchConfig { alpha:f64::NAN,..Default::default() },PhaseResearchConfig { permutations:0,..Default::default() }] {
        assert!(compare(&clock(10),Method::Pelt,&cfg).is_err());
    }
    let mut obs=clock(10); obs[3].pts=f64::NAN;
    assert!(compare(&obs,Method::Fixed,&Default::default()).is_err());
    obs=clock(10); obs[3].frame=0;
    assert!(compare(&obs,Method::Pcid,&Default::default()).is_err());
    obs=clock(10); obs[3].label=Label::Fire {sheet:2,row:0,mirrored:false};
    assert!(compare(&obs,Method::Pelt,&Default::default()).is_err());
}
#[test]
fn pcid_is_seed_reproducible_and_sheet_normalisation_is_shared() {
    let mut obs=phases(&(0..60).map(|i|if i<30 {0.0}else{7.0}).collect::<Vec<_>>());
    let a=compare(&obs,Method::Pcid,&Default::default()).unwrap();
    assert_eq!(a,compare(&obs,Method::Pcid,&Default::default()).unwrap());
    for o in &mut obs { o.label=Label::Fire {sheet:0,row:0,mirrored:true}; }
    assert_eq!(a,compare(&obs,Method::Pcid,&Default::default()).unwrap());
}
#[test]
fn wrapped_rate_diagnostic_recovers_sustained_drift_without_changing_the_check() {
    let obs:Vec<FrameObs>=(0..6000).map(|i| {let t=i as f64/20.0; FrameObs {frame:i,pts:t,label:Label::Fire {sheet:1,row:((20.01*t+0.37).floor() as u64%32) as u8,mirrored:false}}}).collect();
    let fit=clock_diagnostic(&obs,&Default::default()).unwrap().unwrap();
    assert!((fit.estimated_rate-20.01).abs()<0.002,"{fit:?}");
}

#[test]
fn a_cut_inside_a_time_bin_is_refined_to_its_first_observed_frame() {
    let mut obs=clock(900);
    for o in &mut obs[173..] {if let Label::Fire {sheet,row,mirrored}=o.label {o.label=Label::Fire {sheet,row:(row+5)%32,mirrored};}}
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        let a=compare(&obs,method,&Default::default()).unwrap();
        assert!(a.boundaries.iter().any(|&f|f==173),"{method:?}: {:?}",a.boundaries);
    }
}

#[test]
fn global_clock_does_not_follow_a_sequence_of_small_local_offsets() {
    let xs:Vec<f64>=(0..200).map(|i| if i<40 {0.0} else {0.2*((i-40)/40+1) as f64}).collect();
    for method in [Method::Fixed,Method::Pelt,Method::Pcid] {
        let a=compare(&phases(&xs),method,&Default::default()).unwrap();
        assert!(delta(a.reference_phase,0.0).abs()<1e-6);
        assert!((a.clock_residuals.last().unwrap().phase_delta-0.8).abs()<1e-6,"{method:?}");
    }
}

#[test]
fn promotion_gate_binning_can_hide_a_single_wrong_label_that_legacy_catches() {
    let mut obs=clock(900);
    if let Label::Fire {sheet,row,mirrored}=obs[300].label {obs[300].label=Label::Fire {sheet,row:(row+12)%32,mirrored};}
    let legacy=studio_core::phase::analyze(&obs,&Default::default()).unwrap();
    assert!(legacy.flags.iter().any(|f|f.frame==300));
    let binned=compare(&obs,Method::Pelt,&Default::default()).unwrap();
    assert!(!binned.events.iter().any(|e|e.first_frame<=300&&e.last_frame>=300));
}
