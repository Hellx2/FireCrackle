//! First-person on-fire overlay: known screen geometry, so plain sprite matching can name the row.

use fire_core::overlay::{match_overlay, overlay_quads, project};

const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

// ---- geometry ----

#[test]
fn corners_match_hand_computed_values_fov_90_square() {
    // computed independently in python from the vanilla transform (rotate about Y, translate, perspective)
    let q = overlay_quads();
    let p = |quad: usize, i: usize| project(q[quad][i], 90.0, 100, 100).unwrap();
    let want0 = [
        (35.707, 119.057),
        (150.995, 148.624),
        (150.995, 25.344),
        (35.707, 32.736),
    ];
    let want1 = [
        (-50.995, 148.624),
        (64.293, 119.057),
        (64.293, 32.736),
        (-50.995, 25.344),
    ];
    for i in 0..4 {
        let (x, y) = p(0, i);
        assert!(
            close(x, want0[i].0) && close(y, want0[i].1),
            "quad 0 corner {i}: {x},{y}"
        );
        let (x, y) = p(1, i);
        assert!(
            close(x, want1[i].0) && close(y, want1[i].1),
            "quad 1 corner {i}: {x},{y}"
        );
    }
}

#[test]
fn the_two_quads_mirror_each_other_about_the_screen_centre() {
    let q = overlay_quads();
    for &(w, h, fov) in &[
        (1920usize, 1080usize, 70.0),
        (2560, 1440, 90.0),
        (800, 600, 110.0),
    ] {
        for (i, j) in [(0usize, 1usize), (1, 0), (2, 3), (3, 2)] {
            let a = project(q[0][i], fov, w, h).unwrap();
            let b = project(q[1][j], fov, w, h).unwrap();
            assert!(
                close(a.0, w as f64 - b.0) && close(a.1, b.1),
                "{w}x{h} fov {fov} corner {i}"
            );
        }
    }
}

#[test]
fn the_overlay_sits_in_the_lower_part_of_the_screen() {
    let q = overlay_quads();
    let (w, h) = (1920usize, 1080usize);
    let ys: Vec<f64> = q[0]
        .iter()
        .map(|&c| project(c, 70.0, w, h).unwrap().1)
        .collect();
    // the bottom edge is below the frame, the top edge is above the middle... and bottom corners are lower than top ones
    assert!(ys[0] > h as f64 && ys[1] > h as f64);
    assert!(ys[2] < ys[1] && ys[3] < ys[0]);
}

#[test]
fn projection_rejects_points_behind_the_camera_and_bad_fov() {
    assert!(project([0.0, 0.0, 0.5], 70.0, 100, 100).is_none());
    assert!(project([0.0, 0.0, 0.0], 70.0, 100, 100).is_none());
    assert!(project([0.0, 0.0, -1.0], 0.0, 100, 100).is_none());
    assert!(project([0.0, 0.0, -1.0], 180.0, 100, 100).is_none());
    assert!(project([0.0, 0.0, -1.0], f64::NAN, 100, 100).is_none());
    assert!(project([0.0, 0.0, -1.0], 70.0, 0, 100).is_none());
    let (x, y) = project([0.0, 0.0, -1.0], 70.0, 200, 100).unwrap();
    assert!(
        close(x, 100.0) && close(y, 50.0),
        "the optical axis hits the centre"
    );
}

// ---- rendering a synthetic overlay with the same geometry (sub-sampled forward render) ----

fn rnd(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u32
}

fn background(w: usize, h: usize, seed: u64) -> Vec<u8> {
    let mut s = seed;
    let mut f = vec![0u8; w * h * 3];
    for p in f.chunks_mut(3) {
        let v = 30 + (rnd(&mut s) % 70) as u8;
        p.copy_from_slice(&[v, v, v]);
    }
    f
}

/// Draws fire_1 row `row` on both overlay quads. Every texel is painted by dense sub-sampling.
fn render(row: usize, w: usize, h: usize, fov: f64, gain: f64, bg_seed: u64) -> Vec<u8> {
    let mut frame = background(w, h, bg_seed);
    let q = overlay_quads();
    let sub = 40usize;
    for quad in &q {
        // corner order: c0 (s=0,t=0) uv(1,1); c1 (s=1,t=0) uv(0,1); c3 (s=0,t=1) uv(1,0)
        for ty in 0..16 {
            for tx in 0..16 {
                let i = ((row * 16 + ty) * 16 + tx) * 4;
                if S1[i + 3] == 0 {
                    continue;
                }
                for sy in 0..sub {
                    for sx in 0..sub {
                        let u = (tx as f64 + (sx as f64 + 0.5) / sub as f64) / 16.0;
                        let v = (ty as f64 + (sy as f64 + 0.5) / sub as f64) / 16.0;
                        let (s, t) = (1.0 - u, 1.0 - v);
                        let mut p = [0.0; 3];
                        for k in 0..3 {
                            p[k] = quad[0][k]
                                + s * (quad[1][k] - quad[0][k])
                                + t * (quad[3][k] - quad[0][k]);
                        }
                        if let Some((x, y)) = project(p, fov, w, h) {
                            let (xi, yi) = (x.floor(), y.floor());
                            if xi >= 0.0 && yi >= 0.0 && (xi as usize) < w && (yi as usize) < h {
                                let o = (yi as usize * w + xi as usize) * 3;
                                for k in 0..3 {
                                    frame[o + k] =
                                        (S1[i + k] as f64 * gain).clamp(0.0, 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    frame
}

const W: usize = 480;
const H: usize = 270;
const FOV: f64 = 70.0;

#[test]
fn a_rendered_overlay_is_matched_to_its_row_for_every_row() {
    let mut wrong = Vec::new();
    for row in 0..32 {
        let f = render(row, W, H, FOV, 1.0, row as u64);
        let r = match_overlay(S1, &f, W, H, FOV).unwrap();
        assert_eq!(r.len(), 32);
        if r[0].row as usize != row {
            // a row whose visible part is identical to another is not an error
            wrong.push((row, r[0].row, r[0].score));
        }
        assert!(
            r[0].z > 5.0,
            "row {row}: best z {} (r {}, {} samples)",
            r[0].z,
            r[0].score,
            r[0].samples
        );
    }
    assert!(wrong.len() <= 1, "misclassified rows: {wrong:?}");
}

#[test]
fn the_results_are_sorted_best_first_with_enough_samples() {
    let f = render(7, W, H, FOV, 1.0, 1);
    let r = match_overlay(S1, &f, W, H, FOV).unwrap();
    assert!(r.windows(2).all(|p| p[0].z >= p[1].z));
    assert!(r[0].samples >= 10);
}

#[test]
fn a_dark_veil_keeps_the_row() {
    for gain in [0.6, 0.3] {
        let f = render(12, W, H, FOV, gain, 3);
        assert_eq!(
            match_overlay(S1, &f, W, H, FOV).unwrap()[0].row,
            12,
            "gain {gain}"
        );
    }
}

#[test]
fn a_gui_bar_over_the_bottom_centre_does_not_break_it() {
    let mut f = render(20, W, H, FOV, 1.0, 4);
    // a vanilla hotbar is 182x22 GUI px; at GUI scale 3 on 1080p that is 546x66 px, i.e. about 137x17 px at this 480x270
    for y in (H - 17)..H {
        for x in (W / 2 - 68)..(W / 2 + 69) {
            f[(y * W + x) * 3..(y * W + x) * 3 + 3].copy_from_slice(&[60, 60, 60]);
        }
    }
    assert_eq!(match_overlay(S1, &f, W, H, FOV).unwrap()[0].row, 20);
}

#[test]
fn a_frame_without_the_overlay_scores_low() {
    let mut worst = f64::NEG_INFINITY;
    for seed in 0..10 {
        let f = background(W, H, 100 + seed);
        let r = match_overlay(S1, &f, W, H, FOV).unwrap();
        worst = worst.max(r[0].z);
    }
    // only ~10-45 texels per row are on screen (the overlay is mostly below the frame), so raw correlation is noisy.
    // z = atanh(r) * sqrt(n - 3) is ~N(0,1) on noise whatever n is: the best of 32 rows stays well under 4 sigma.
    assert!(
        worst < 4.0,
        "noise should not look like the overlay: z = {worst}"
    );
}

#[test]
fn a_flat_frame_has_no_signal_and_no_nan() {
    let f = vec![80u8; W * H * 3];
    let r = match_overlay(S1, &f, W, H, FOV).unwrap();
    assert!(r
        .iter()
        .all(|s| !s.score.is_nan() && !s.z.is_nan() && s.score <= 0.0 && s.z <= 0.0));
}

#[test]
fn input_errors() {
    let f = background(W, H, 1);
    assert!(match_overlay(S1, &f[..f.len() - 3], W, H, FOV).is_err());
    assert!(match_overlay(&S1[..100], &f, W, H, FOV).is_err());
    assert!(match_overlay(S1, &f, W, H, 0.0).is_err());
    assert!(match_overlay(S1, &f, W, H, 200.0).is_err());
    assert!(match_overlay(S1, &[], 0, 0, FOV).is_err());
}

#[test]
fn a_wrong_fov_scores_clearly_lower_than_the_right_one() {
    let f = render(9, W, H, FOV, 1.0, 5);
    let right = match_overlay(S1, &f, W, H, FOV).unwrap()[0].z;
    let wrong = match_overlay(S1, &f, W, H, 95.0).unwrap()[0].z;
    assert!(right > wrong + 2.0, "right {right}, wrong {wrong}");
}
