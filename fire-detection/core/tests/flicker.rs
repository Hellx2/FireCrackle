use fire_core::flicker::{bg_motion, find_partner, flicker_mask};
use fire_core::presence::Palette;

const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn pal() -> Palette {
    Palette::from_rgba(&[S0, S1], 8.0).unwrap()
}

fn luma(c: &[u8; 3]) -> f64 {
    0.299 * c[0] as f64 + 0.587 * c[1] as f64 + 0.114 * c[2] as f64
}

/// The darkest and brightest fire colours and their luma gap.
fn two_fire_colours() -> ([u8; 3], [u8; 3], f64) {
    let cols: Vec<[u8; 3]> = S0
        .chunks(4)
        .filter(|c| c[3] != 0)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    let dark = *cols
        .iter()
        .min_by(|a, b| luma(a).partial_cmp(&luma(b)).unwrap())
        .unwrap();
    let bright = *cols
        .iter()
        .max_by(|a, b| luma(a).partial_cmp(&luma(b)).unwrap())
        .unwrap();
    (dark, bright, luma(&bright) - luma(&dark))
}

fn solid(w: usize, h: usize, c: [u8; 3]) -> Vec<u8> {
    c.iter().copied().cycle().take(w * h * 3).collect()
}

fn set(f: &mut [u8], w: usize, x: usize, y: usize, c: [u8; 3]) {
    f[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&c);
}

#[test]
fn identical_frames_have_no_flicker() {
    let (dark, _, _) = two_fire_colours();
    let f = solid(8, 8, dark);
    let m = flicker_mask(&f, &f, 8, 8, 0, &pal(), 60.0).unwrap();
    assert_eq!(m.len(), 64);
    assert!(m.iter().all(|&b| !b));
}

#[test]
fn a_fire_pixel_that_changes_shade_flickers_and_only_that_pixel() {
    let (dark, bright, gap) = two_fire_colours();
    assert!(gap > 100.0, "the fire ramp spans a big luma range: {gap}");
    let a = solid(8, 8, dark);
    let mut b = a.clone();
    set(&mut b, 8, 3, 5, bright);
    let m = flicker_mask(&a, &b, 8, 8, 0, &pal(), 60.0).unwrap();
    assert_eq!(m.iter().filter(|&&x| x).count(), 1);
    assert!(m[5 * 8 + 3]);
}

#[test]
fn the_luma_threshold_is_inclusive() {
    let (dark, bright, gap) = two_fire_colours();
    let a = solid(2, 1, dark);
    let mut b = a.clone();
    set(&mut b, 2, 0, 0, bright);
    assert!(flicker_mask(&a, &b, 2, 1, 0, &pal(), gap - 1e-6).unwrap()[0]); // just under the gap: counts
    assert!(!flicker_mask(&a, &b, 2, 1, 0, &pal(), gap + 0.5).unwrap()[0]);
}

#[test]
fn a_pixel_orange_in_only_one_frame_does_not_count() {
    let (dark, _, _) = two_fire_colours();
    let a = solid(4, 4, dark);
    let b = solid(4, 4, [10, 10, 200]);
    assert!(flicker_mask(&a, &b, 4, 4, 0, &pal(), 10.0)
        .unwrap()
        .iter()
        .all(|&x| !x));
    assert!(flicker_mask(&b, &a, 4, 4, 0, &pal(), 10.0)
        .unwrap()
        .iter()
        .all(|&x| !x));
}

#[test]
fn non_palette_changes_never_flicker() {
    let p = pal();
    let a = solid(4, 4, [0, 0, 0]);
    let b = solid(4, 4, [10, 10, 200]);
    assert_eq!(
        p.count(&a, 4, 4).unwrap() + p.count(&b, 4, 4).unwrap(),
        0,
        "premise: neither is fire coloured"
    );
    assert!(flicker_mask(&a, &b, 4, 4, 0, &p, 10.0)
        .unwrap()
        .iter()
        .all(|&x| !x));
}

#[test]
fn rows_above_y0_are_left_out_of_the_mask() {
    let (dark, bright, _) = two_fire_colours();
    let m = flicker_mask(
        &solid(4, 6, dark),
        &solid(4, 6, bright),
        4,
        6,
        2,
        &pal(),
        60.0,
    )
    .unwrap();
    assert_eq!(m.len(), 4 * 4);
    assert!(m.iter().all(|&x| x));
}

#[test]
fn flicker_input_errors() {
    let (dark, _, _) = two_fire_colours();
    let a = solid(4, 4, dark);
    let p = pal();
    assert!(flicker_mask(&a, &a[..a.len() - 3], 4, 4, 0, &p, 10.0).is_err());
    assert!(flicker_mask(&a[..a.len() - 3], &a, 4, 4, 0, &p, 10.0).is_err());
    assert!(flicker_mask(&a, &a, 4, 4, 5, &p, 10.0).is_err());
    assert!(flicker_mask(&[], &[], 0, 0, 0, &p, 10.0).is_err());
    assert!(flicker_mask(&a, &a, 4, 4, 0, &p, f64::NAN).is_err());
}

// ---- background motion ----

#[test]
fn background_motion_is_zero_for_identical_frames_and_measures_non_palette_change() {
    let p = pal();
    let a = solid(8, 8, [20, 20, 20]);
    assert_eq!(bg_motion(&a, &a, 8, 8, 0, &p, 1).unwrap(), 0.0);
    let b = solid(8, 8, [50, 50, 50]);
    assert!((bg_motion(&a, &b, 8, 8, 0, &p, 1).unwrap() - 30.0).abs() < 0.5);
}

#[test]
fn background_motion_ignores_palette_pixels() {
    let (dark, bright, _) = two_fire_colours();
    let p = pal();
    // all fire in both frames, changing a lot: no background pixels -> 0, not NaN
    assert_eq!(
        bg_motion(&solid(4, 4, dark), &solid(4, 4, bright), 4, 4, 0, &p, 1).unwrap(),
        0.0
    );
    // top half fire (changing), bottom half grey and unchanged -> 0
    let mut a = solid(4, 4, [30, 30, 30]);
    let mut b = a.clone();
    for x in 0..4 {
        for y in 0..2 {
            set(&mut a, 4, x, y, dark);
            set(&mut b, 4, x, y, bright);
        }
    }
    assert_eq!(bg_motion(&a, &b, 4, 4, 0, &p, 1).unwrap(), 0.0);
}

#[test]
fn background_motion_sampling_step_and_errors() {
    let p = pal();
    let a = solid(8, 8, [20, 20, 20]);
    let b = solid(8, 8, [50, 50, 50]);
    assert!((bg_motion(&a, &b, 8, 8, 0, &p, 4).unwrap() - 30.0).abs() < 0.5);
    assert!(bg_motion(&a, &b, 8, 8, 0, &p, 0).is_err());
    assert!(bg_motion(&a, &b[..b.len() - 3], 8, 8, 0, &p, 1).is_err());
    assert!(bg_motion(&a, &b, 8, 8, 9, &p, 1).is_err());
}

// ---- partner search ----

#[test]
fn the_partner_is_the_latest_frame_of_the_previous_tick() {
    let hist = [(10u64, 5u64), (11, 5), (12, 6), (13, 6)];
    assert_eq!(find_partner(&hist, 7), Some(3)); // tick 6: latest entry
    assert_eq!(find_partner(&hist, 6), Some(1)); // tick 5: latest entry
}

#[test]
fn no_partner_when_the_previous_tick_is_missing_or_only_the_same_tick_exists() {
    assert_eq!(find_partner(&[(1, 5), (2, 5)], 5), None);
    assert_eq!(find_partner(&[(1, 3), (2, 3)], 5), None); // tick 4 missing: a gap
    assert_eq!(find_partner(&[], 5), None);
}

#[test]
fn tick_zero_has_no_predecessor_and_does_not_underflow() {
    assert_eq!(find_partner(&[(1, 0)], 0), None);
}
