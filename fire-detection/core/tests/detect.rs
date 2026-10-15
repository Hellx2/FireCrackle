//! Template matcher: renders real fire textures into synthetic frames and checks they are recognised.

use fire_core::detect::{classify, Candidate, Templates};

const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn templates() -> Templates {
    Templates::from_rgba(S0, S1).unwrap()
}

/// Cheap deterministic noise.
fn rnd(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u32
}

struct Scene {
    frame: Vec<u8>,
    w: usize,
    h: usize,
    /// bbox of the drawn opaque texels, label style [x0,y0,x1,y1]
    bbox: [f64; 4],
}

/// Draws one candidate at integer scale `s` (nearest neighbour) onto a noisy grey background.
fn render(c: Candidate, s: usize, gain: f64, noise: u32, at: (usize, usize)) -> Scene {
    let (w, h) = (200usize, 160usize);
    let mut seed = 12345u64 + c.row as u64;
    let mut frame = vec![0u8; w * h * 3];
    for p in frame.chunks_mut(3) {
        let v = 40 + (rnd(&mut seed) % 60) as u8;
        p.copy_from_slice(&[v, v, v]);
    }
    let sheet = if c.sheet == 0 { S0 } else { S1 };
    let (mut minx, mut miny, mut maxx, mut maxy) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for ty in 0..16 {
        for tx in 0..16 {
            let sx = if c.mirror { 15 - tx } else { tx };
            let i = ((c.row as usize * 16 + ty) * 16 + sx) * 4;
            if sheet[i + 3] == 0 {
                continue;
            }
            for dy in 0..s {
                for dx in 0..s {
                    let x = at.0 + tx * s + dx;
                    let y = at.1 + ty * s + dy;
                    let o = (y * w + x) * 3;
                    for k in 0..3 {
                        let mut v = sheet[i + k] as f64 * gain;
                        if noise > 0 {
                            v += (rnd(&mut seed) % (2 * noise + 1)) as f64 - noise as f64;
                        }
                        frame[o + k] = v.clamp(0.0, 255.0) as u8;
                    }
                }
            }
            minx = minx.min(at.0 + tx * s);
            miny = miny.min(at.1 + ty * s);
            maxx = maxx.max(at.0 + (tx + 1) * s);
            maxy = maxy.max(at.1 + (ty + 1) * s);
        }
    }
    Scene {
        frame,
        w,
        h,
        bbox: [minx as f64, miny as f64, maxx as f64, maxy as f64],
    }
}

fn cand(sheet: u8, mirror: bool, row: u8) -> Candidate {
    Candidate { sheet, mirror, row }
}

fn best(t: &Templates, sc: &Scene) -> Candidate {
    classify(t, &sc.frame, sc.w, sc.h, sc.bbox).unwrap()[0].cand
}

/// Candidates whose opaque pixels are identical to an earlier one cannot be told apart by any matcher.
fn indistinguishable(t: &Templates) -> Vec<Candidate> {
    let all = t.candidates();
    let mut dup = Vec::new();
    for (i, a) in all.iter().enumerate() {
        for b in &all[..i] {
            if t.same_pixels(*a, *b) {
                dup.push(*a);
                break;
            }
        }
    }
    dup
}

// ---- basics ----

#[test]
fn there_are_128_candidates() {
    let t = templates();
    assert_eq!(t.candidates().len(), 128);
    assert_eq!(
        classify(
            &t,
            &render(cand(0, false, 3), 3, 1.0, 0, (20, 20)).frame,
            200,
            160,
            [20.0, 20.0, 60.0, 60.0]
        )
        .unwrap()
        .len(),
        128
    );
}

#[test]
fn every_candidate_is_recognised_at_3x() {
    let t = templates();
    let dups = indistinguishable(&t);
    let mut wrong = Vec::new();
    for c in t.candidates() {
        if dups.contains(&c) {
            continue;
        }
        let sc = render(c, 3, 1.0, 0, (30, 20));
        let got = best(&t, &sc);
        if got != c && !t.same_pixels(got, c) {
            wrong.push((c, got));
        }
    }
    assert!(wrong.is_empty(), "misclassified: {wrong:?}");
}

#[test]
fn mirrored_is_told_apart_from_normal() {
    let t = templates();
    let dups = indistinguishable(&t);
    let mut checked = 0;
    for row in 0..32u8 {
        let n = cand(0, false, row);
        let m = cand(0, true, row);
        if dups.contains(&n) || dups.contains(&m) || t.same_pixels(n, m) {
            continue; // a left-right symmetric frame
        }
        assert_eq!(
            best(&t, &render(m, 3, 1.0, 0, (10, 10))),
            m,
            "row {row} mirrored"
        );
        assert_eq!(
            best(&t, &render(n, 3, 1.0, 0, (10, 10))),
            n,
            "row {row} normal"
        );
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} asymmetric rows");
}

#[test]
fn scale_does_not_matter() {
    let t = templates();
    let c = cand(1, true, 9);
    for s in [1usize, 2, 3, 5, 7] {
        assert_eq!(best(&t, &render(c, s, 1.0, 0, (5, 5))), c, "scale {s}");
    }
}

#[test]
fn position_does_not_matter() {
    let t = templates();
    let c = cand(0, false, 20);
    for at in [(0usize, 0usize), (50, 30), (80, 40)] {
        assert_eq!(best(&t, &render(c, 3, 1.0, 0, at)), c, "at {at:?}");
    }
}

#[test]
fn scores_are_sorted_best_first_and_the_truth_scores_near_one() {
    let t = templates();
    let c = cand(1, false, 14);
    let sc = render(c, 3, 1.0, 0, (30, 20));
    let r = classify(&t, &sc.frame, sc.w, sc.h, sc.bbox).unwrap();
    assert!(r.windows(2).all(|p| p[0].score >= p[1].score));
    assert_eq!(r[0].cand, c);
    assert!(r[0].score > 0.95, "{}", r[0].score);
}

// ---- robustness ----

#[test]
fn a_dark_veil_keeps_the_answer() {
    let t = templates();
    let c = cand(0, true, 5);
    for gain in [0.7, 0.5, 0.25] {
        assert_eq!(best(&t, &render(c, 3, gain, 0, (30, 20))), c, "gain {gain}");
    }
}

#[test]
fn moderate_noise_keeps_the_answer() {
    let t = templates();
    let dups = indistinguishable(&t);
    let mut bad = 0;
    let mut n = 0;
    for c in t.candidates() {
        if dups.contains(&c) {
            continue;
        }
        n += 1;
        let got = best(&t, &render(c, 3, 1.0, 12, (30, 20)));
        if got != c && !t.same_pixels(got, c) {
            bad += 1;
        }
    }
    assert!(bad * 20 <= n, "{bad} of {n} wrong with noise 12");
}

#[test]
fn a_box_partly_off_the_frame_is_clamped_not_rejected() {
    let t = templates();
    let sc = render(cand(0, false, 2), 3, 1.0, 0, (30, 20));
    let b = [
        sc.bbox[0] - 500.0,
        sc.bbox[1],
        sc.bbox[2],
        sc.bbox[3] + 500.0,
    ];
    assert!(classify(&t, &sc.frame, sc.w, sc.h, b).is_ok());
}

#[test]
fn a_one_pixel_wide_box_does_not_panic_or_produce_nan() {
    let t = templates();
    let sc = render(cand(0, false, 2), 3, 1.0, 0, (30, 20));
    for b in [
        [50.0, 30.0, 51.0, 31.0],
        [50.0, 30.0, 51.5, 60.0],
        [50.2, 30.3, 50.9, 30.8],
    ] {
        let r = classify(&t, &sc.frame, sc.w, sc.h, b).unwrap();
        assert_eq!(r.len(), 128);
        assert!(r.iter().all(|s| !s.score.is_nan()));
    }
}

#[test]
fn a_flat_crop_has_no_signal_and_no_nan() {
    let t = templates();
    let (w, h) = (64usize, 64usize);
    let frame = vec![90u8; w * h * 3];
    let r = classify(&t, &frame, w, h, [10.0, 10.0, 40.0, 40.0]).unwrap();
    assert_eq!(r.len(), 128);
    assert!(r.iter().all(|s| !s.score.is_nan() && s.score <= 0.0));
    // deterministic tie-break: lowest candidate first
    assert_eq!(r[0].cand, t.candidates()[0]);
}

#[test]
fn bad_boxes_are_errors() {
    let t = templates();
    let frame = vec![0u8; 64 * 64 * 3];
    for b in [
        [10.0, 10.0, 10.0, 20.0],
        [10.0, 10.0, 20.0, 10.0],
        [20.0, 10.0, 10.0, 20.0],
        [f64::NAN, 0.0, 10.0, 10.0],
        [0.0, 0.0, f64::INFINITY, 10.0],
        [100.0, 100.0, 120.0, 120.0], // wholly outside
        [-50.0, -50.0, -10.0, -10.0],
    ] {
        assert!(classify(&t, &frame, 64, 64, b).is_err(), "{b:?}");
    }
}

#[test]
fn a_frame_of_the_wrong_length_is_an_error() {
    let t = templates();
    assert!(classify(
        &t,
        &vec![0u8; 64 * 64 * 3 - 1],
        64,
        64,
        [0.0, 0.0, 10.0, 10.0]
    )
    .is_err());
    assert!(classify(&t, &vec![0u8; 64 * 64], 64, 64, [0.0, 0.0, 10.0, 10.0]).is_err());
}

#[test]
fn bad_texture_sheets_are_rejected() {
    assert!(Templates::from_rgba(&S0[..100], S1).is_err());
    assert!(Templates::from_rgba(S0, &[]).is_err());
}

#[test]
fn the_input_frame_is_not_modified() {
    let t = templates();
    let sc = render(cand(0, false, 2), 3, 1.0, 5, (30, 20));
    let copy = sc.frame.clone();
    let _ = classify(&t, &sc.frame, sc.w, sc.h, sc.bbox).unwrap();
    assert_eq!(sc.frame, copy);
}
