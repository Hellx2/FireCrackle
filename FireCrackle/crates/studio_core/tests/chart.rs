use studio_core::chart::decimate;
use studio_core::phase::Point;

fn pt(frame: u32, phase: f64) -> Point {
    Point {
        frame,
        pts: frame as f64 / 20.0,
        step: (frame % 32) as u8,
        phase,
    }
}

fn flat(n: u32, phase: f64) -> Vec<Point> {
    (0..n).map(|f| pt(f, phase)).collect()
}

fn frames(p: &[Point]) -> Vec<u32> {
    p.iter().map(|p| p.frame).collect()
}

// ------------------------------------------------------------ basics

#[test]
fn fewer_points_than_the_cap_come_back_unchanged() {
    let p = flat(10, 3.0);
    assert_eq!(decimate(&p, 100), p);
    assert_eq!(decimate(&p, 10), p, "exactly the cap is unchanged");
}

#[test]
fn an_empty_input_gives_an_empty_output() {
    assert!(decimate(&[], 100).is_empty());
}

#[test]
fn a_big_input_is_reduced_to_at_most_the_cap() {
    let p = flat(10_000, 5.0);
    let d = decimate(&p, 300);
    assert!(!d.is_empty() && d.len() <= 300, "got {}", d.len());
}

#[test]
fn the_output_keeps_frame_order_without_duplicates() {
    let p: Vec<Point> = (0..5_000).map(|f| pt(f, (f % 7) as f64)).collect();
    let d = decimate(&p, 200);
    let f = frames(&d);
    assert!(
        f.windows(2).all(|w| w[0] < w[1]),
        "must be strictly increasing: {f:?}"
    );
}

#[test]
fn the_first_and_last_points_always_survive() {
    let p = flat(5_000, 1.0);
    let d = decimate(&p, 200);
    assert_eq!(d.first().unwrap().frame, 0);
    assert_eq!(d.last().unwrap().frame, 4_999);
}

#[test]
fn every_output_point_is_an_input_point() {
    let p: Vec<Point> = (0..3_000).map(|f| pt(f, (f % 11) as f64)).collect();
    for q in decimate(&p, 150) {
        assert_eq!(q, p[q.frame as usize]);
    }
}

// ------------------------------------------------------------ failure cases

#[test]
fn an_isolated_outlier_survives_decimation() {
    let mut p = flat(10_000, 5.0);
    p[6_543] = pt(6_543, 12.0);
    let d = decimate(&p, 300);
    assert!(
        d.iter().any(|q| q.frame == 6_543),
        "the outlier was averaged away"
    );
}

#[test]
fn a_step_in_the_phase_stays_visible() {
    let p: Vec<Point> = (0..9_000)
        .map(|f| pt(f, if f < 4_500 { 2.0 } else { 9.0 }))
        .collect();
    let d = decimate(&p, 300);
    assert!(d.iter().any(|q| q.phase == 2.0) && d.iter().any(|q| q.phase == 9.0));
}

#[test]
fn extremes_are_circular_not_numeric_at_the_seam() {
    // phases 31.9 and 0.1 are 0.2 ticks apart; with a numeric min/max the 31.9 point would look like a far
    // outlier and crowd out the real one at 6.0 in the same bucket.
    let mut p: Vec<Point> = (0..3_000)
        .map(|f| pt(f, if f % 2 == 0 { 31.9 } else { 0.1 }))
        .collect();
    p[1_500] = pt(1_500, 6.0);
    let d = decimate(&p, 90);
    assert!(
        d.iter().any(|q| q.frame == 1_500),
        "the real outlier must be kept"
    );
}

#[test]
fn a_cap_of_zero_gives_nothing() {
    assert!(decimate(&flat(100, 1.0), 0).is_empty());
}

#[test]
fn tiny_caps_stay_within_the_cap() {
    let p = flat(1_000, 1.0);
    for cap in 1..=4 {
        let d = decimate(&p, cap);
        assert!(d.len() <= cap, "cap {cap} got {}", d.len());
        assert!(!d.is_empty());
    }
    assert_eq!(
        decimate(&p, 2).iter().map(|q| q.frame).collect::<Vec<_>>(),
        vec![0, 999],
        "cap 2 keeps the ends"
    );
}

#[test]
fn a_single_point_is_returned_as_is() {
    let p = flat(1, 4.0);
    assert_eq!(decimate(&p, 10), p);
    assert_eq!(decimate(&p, 1), p);
}

#[test]
fn decimation_is_deterministic() {
    let p: Vec<Point> = (0..4_000).map(|f| pt(f, (f % 13) as f64)).collect();
    assert_eq!(decimate(&p, 250), decimate(&p, 250));
}
