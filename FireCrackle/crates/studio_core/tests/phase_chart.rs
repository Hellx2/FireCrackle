use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use studio_core::config::{IndexConfig, Settings};
use studio_core::decode_plan::DecodeJob;
use studio_core::frames::{DecodeError, DecodedFrame};
use studio_core::index::{build_index, Packet};
use studio_core::label::Label;
use studio_core::phase::FlagKind;
use studio_core::project::{Project, VideoMeta};
use studio_core::workspace::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fc_chart_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn project(n: usize, fps: f64) -> Project {
    let pk: Vec<Packet> = (0..n).map(|i| Packet { pts: Some(i as f64 / fps), dts: None, key: i % 30 == 0 }).collect();
    let idx = build_index(&pk, &IndexConfig::default()).unwrap();
    let meta = VideoMeta { width: 1920, height: 1080, fps_num: fps as u32, fps_den: 1, codec: "h264".into() };
    Project::assemble("v.mkv", 1, "1:4096:0".into(), meta, &idx)
}

struct Fake;
impl FrameSource for Fake {
    fn decode(&self, job: &DecodeJob, _c: &AtomicBool) -> Result<Vec<DecodedFrame>, DecodeError> {
        Ok((job.first..=job.last).map(|f| DecodedFrame { frame: f, jpeg: vec![f as u8; 8] }).collect())
    }
}

fn ws(name: &str, n: usize) -> (PathBuf, Workspace) {
    let dir = scratch(name);
    let w = Workspace::from_parts(&dir, project(n, 20.0), &Settings::default(), Arc::new(Fake)).unwrap();
    (dir, w)
}

/// Label frames 0..n as sheet 1 with the true step (frame i shows row i mod 32, so phi = 0 everywhere), by clicking
/// the picker; every click advances one frame. `skew` rows are added from frame `skew_from` on (a cut).
fn label_run(w: &mut Workspace, n: u32, skew_from: u32, skew: u32) {
    w.apply(UiAction::GoTo { frame: 0 }).unwrap();
    for f in 0..n {
        let row = (f + if f >= skew_from { skew } else { 0 }) % 32;
        w.apply(UiAction::ClickCell { col: 1, row }).unwrap();
    }
}

#[test]
fn an_unlabelled_project_has_an_empty_chart() {
    let (dir, mut w) = ws("empty", 50);
    let c = w.phase_chart(100);
    assert!(c.points.is_empty() && c.runs.is_empty() && c.flags.is_empty());
    assert_eq!(c.tick_rate, 20.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn non_fire_labels_are_not_chart_points() {
    let (dir, mut w) = ws("nofire", 50);
    w.apply(UiAction::NoFire).unwrap();
    w.apply(UiAction::Skip).unwrap();
    assert!(w.phase_chart(100).points.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_single_fire_label_is_one_point_and_no_flags() {
    let (dir, mut w) = ws("one", 50);
    w.apply(UiAction::ClickCell { col: 1, row: 5 }).unwrap();
    let c = w.phase_chart(100);
    assert_eq!(c.points.len(), 1);
    assert_eq!(c.points[0].frame, 0);
    assert!(c.flags.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_clean_run_is_one_flat_run_at_phase_zero() {
    let (dir, mut w) = ws("clean", 64);
    label_run(&mut w, 40, 999, 0);
    let c = w.phase_chart(1000);
    assert_eq!(c.points.len(), 40);
    assert!(c.points.iter().all(|p| p.phase.abs() < 1e-6 || (p.phase - 32.0).abs() < 1e-6), "phi must be 0");
    assert_eq!(c.runs.len(), 1);
    assert!(c.flags.is_empty());
    assert_eq!(c.points[7].step, 7);
    assert!((c.points[7].pts - 7.0 / 20.0).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cut_shows_up_as_a_shift_flag_and_two_runs() {
    let (dir, mut w) = ws("cut", 64);
    label_run(&mut w, 40, 20, 5);
    let c = w.phase_chart(1000);
    assert_eq!(c.runs.len(), 2);
    assert!(c.flags.iter().any(|f| f.kind == FlagKind::PhaseShift && f.frame == 20), "{:?}", c.flags);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_chart_flags_equal_the_flag_list() {
    let (dir, mut w) = ws("same", 64);
    label_run(&mut w, 40, 20, 5);
    let c = w.phase_chart(1000);
    assert_eq!(c.flags, w.flags());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cap_limits_the_points_but_not_the_flags() {
    let (dir, mut w) = ws("cap", 64);
    label_run(&mut w, 60, 30, 5);
    let c = w.phase_chart(10);
    assert!(c.points.len() <= 10, "got {}", c.points.len());
    assert!(!c.flags.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn editing_a_label_updates_the_next_chart() {
    let (dir, mut w) = ws("stale", 64);
    label_run(&mut w, 40, 999, 0);
    assert!(w.phase_chart(1000).flags.is_empty());
    w.apply(UiAction::GoTo { frame: 25 }).unwrap();
    w.apply(UiAction::ClickCell { col: 1, row: 30 }).unwrap(); // frame 25 should show row 25
    let c = w.phase_chart(1000);
    assert!(!c.flags.is_empty(), "the stale chart was served");
    assert_eq!(c.points.iter().find(|p| p.frame == 25).unwrap().step, 30);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changing_a_phase_setting_updates_the_chart() {
    let (dir, mut w) = ws("setting", 64);
    label_run(&mut w, 40, 20, 5);
    let before = w.phase_chart(1000).flags.len();
    assert!(before > 0);
    w.set_setting("phase.min_shift_ticks", 8.0).unwrap(); // 5 ticks is now inside the noise floor
    let after = w.phase_chart(1000);
    assert!(after.flags.len() < before, "{before} -> {}", after.flags.len());
    assert_eq!(after.window_ticks, 2.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_chart_serialises_to_json_the_ui_can_read() {
    let (dir, mut w) = ws("json", 64);
    label_run(&mut w, 40, 20, 5);
    let v = serde_json::to_value(w.phase_chart(1000)).unwrap();
    assert!(v["points"][0]["frame"].is_number());
    assert!(v["points"][0]["pts"].is_number());
    assert!(v["points"][0]["step"].is_number());
    assert!(v["points"][0]["phase"].is_number());
    assert!(v["runs"][0]["first_frame"].is_number());
    assert!(v["runs"][0]["center"].is_number());
    assert_eq!(v["flags"][0]["kind"], "phase_shift");
    assert_eq!(v["tick_rate"], 20.0);
    let _ = Label::Unset;
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_session_chart_includes_the_fixed_clock_and_event_evidence() {
    let (dir,mut w)=ws("clock_evidence",64);label_run(&mut w,40,20,5);
    let v=serde_json::to_value(w.phase_chart(1000)).unwrap();
    assert!(v["reference_phase"].is_number());assert!(v["flag_details"].as_array().is_some_and(|a|!a.is_empty()));
    assert!(v["clock_residuals"].as_array().is_some_and(|a|!a.is_empty()));
    let _=std::fs::remove_dir_all(&dir);
}
