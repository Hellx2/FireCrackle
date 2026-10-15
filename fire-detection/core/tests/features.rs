use fire_core::features::{blob_stats, mask_edge_stats, mean_luma_and_dark, unveil_rows};
use fire_core::presence::Palette;

fn mask_from(rows: &[&str]) -> (Vec<bool>, usize, usize) {
    let h = rows.len();
    let w = rows[0].len();
    let mut m = Vec::new();
    for r in rows {
        assert_eq!(r.len(), w);
        m.extend(r.bytes().map(|c| c == b'#'));
    }
    (m, w, h)
}

// ---- blobs ----

#[test]
fn one_solid_blob() {
    let (m, w, h) = mask_from(&["....", ".##.", ".##.", "...."]);
    let b = blob_stats(&m, w, h, 3).unwrap();
    assert_eq!((b.total, b.blobs_ge_min, b.largest, b.in_big), (4, 1, 4, 4));
}

#[test]
fn two_separate_blobs_are_counted_apart() {
    let (m, w, h) = mask_from(&["##...", "##...", ".....", "...##", "...##"]);
    let b = blob_stats(&m, w, h, 4).unwrap();
    assert_eq!((b.blobs_ge_min, b.largest, b.total), (2, 4, 8));
}

#[test]
fn diagonal_pixels_are_not_connected() {
    let (m, w, h) = mask_from(&["#...", ".#..", "..#.", "...#"]);
    let b = blob_stats(&m, w, h, 2).unwrap();
    assert_eq!((b.blobs_ge_min, b.largest, b.in_big), (0, 1, 0));
}

#[test]
fn a_blob_touching_the_border_and_a_snake_are_one_blob() {
    let (m, w, h) = mask_from(&["####", "...#", "####", "#...", "####"]);
    let b = blob_stats(&m, w, h, 5).unwrap();
    assert_eq!((b.blobs_ge_min, b.largest, b.total), (1, 4 + 1 + 4 + 1 + 4, 14));
}

#[test]
fn the_min_area_edge_is_inclusive() {
    let (m, w, h) = mask_from(&["###..", ".....", "##..."]);
    assert_eq!(blob_stats(&m, w, h, 3).unwrap().blobs_ge_min, 1);
    assert_eq!(blob_stats(&m, w, h, 4).unwrap().blobs_ge_min, 0);
    assert_eq!(blob_stats(&m, w, h, 2).unwrap().blobs_ge_min, 2);
    assert_eq!(blob_stats(&m, w, h, 2).unwrap().in_big, 5);
}

#[test]
fn empty_and_full_masks() {
    let e = blob_stats(&vec![false; 12], 4, 3, 1).unwrap();
    assert_eq!((e.total, e.blobs_ge_min, e.largest, e.in_big), (0, 0, 0, 0));
    let f = blob_stats(&vec![true; 12], 4, 3, 1).unwrap();
    assert_eq!((f.total, f.blobs_ge_min, f.largest, f.in_big), (12, 1, 12, 12));
}

#[test]
fn blob_input_errors() {
    assert!(blob_stats(&[true; 5], 2, 2, 1).is_err());
    assert!(blob_stats(&[], 0, 0, 1).is_err());
    assert!(blob_stats(&[true; 4], 2, 2, 0).is_err());
}

#[test]
fn a_full_hd_mask_is_fast_and_does_not_overflow_the_stack() {
    let (w, h) = (1920usize, 1080usize);
    let mut m = vec![true; w * h]; // one huge blob: a recursive fill would overflow
    for y in (0..h).step_by(7) {
        m[y * w + 5] = false; // some holes
    }
    let t = std::time::Instant::now();
    let b = blob_stats(&m, w, h, 10).unwrap();
    assert_eq!(b.blobs_ge_min, 1);
    assert!(t.elapsed().as_secs_f64() < 2.0, "{:?}", t.elapsed());
}

// ---- edge stats ----

#[test]
fn edge_stats_of_a_flat_and_a_noisy_row() {
    let luma = [100.0, 100.0, 100.0, 100.0];
    let s = mask_edge_stats(&[true; 4], &luma, 4, 1).unwrap();
    assert_eq!((s.pairs, s.mean_abs_diff, s.share_big), (3, 0.0, 0.0));
    let luma = [0.0, 100.0, 0.0, 100.0];
    let s = mask_edge_stats(&[true; 4], &luma, 4, 1).unwrap();
    assert_eq!(s.pairs, 3);
    assert_eq!(s.mean_abs_diff, 100.0);
    assert_eq!(s.share_big, 1.0);
}

#[test]
fn edge_stats_only_count_pairs_where_both_pixels_are_in_the_mask() {
    let luma = [0.0, 50.0, 200.0, 0.0];
    let s = mask_edge_stats(&[true, true, false, true], &luma, 4, 1).unwrap();
    assert_eq!(s.pairs, 1);
    assert_eq!(s.mean_abs_diff, 50.0);
}

#[test]
fn edge_stats_do_not_pair_across_row_ends_and_are_zero_not_nan_when_empty() {
    let luma = [0.0, 0.0, 255.0, 255.0];
    // 2 wide, 2 tall: the pair (x=1,y=0)-(x=0,y=1) must not be formed
    let s = mask_edge_stats(&[true; 4], &luma, 2, 2).unwrap();
    assert_eq!(s.pairs, 2);
    assert_eq!(s.mean_abs_diff, 0.0);
    let s = mask_edge_stats(&[false; 4], &luma, 2, 2).unwrap();
    assert_eq!((s.pairs, s.mean_abs_diff, s.share_big), (0, 0.0, 0.0));
    assert!(mask_edge_stats(&[true; 3], &luma, 2, 2).is_err());
    assert!(mask_edge_stats(&[true; 4], &luma[..3], 2, 2).is_err());
}

// ---- brightness ----

#[test]
fn mean_luma_and_dark_share() {
    let rgb = [0u8, 0, 0, 255, 255, 255, 10, 10, 10, 200, 200, 200];
    let (mean, dark) = mean_luma_and_dark(&rgb, 4, 1, 30.0).unwrap();
    assert!((mean - (0.0 + 255.0 + 10.0 + 200.0) / 4.0).abs() < 0.5, "{mean}");
    assert_eq!(dark, 0.5);
    assert!(mean_luma_and_dark(&rgb, 3, 1, 30.0).is_err());
    assert!(mean_luma_and_dark(&[], 0, 0, 30.0).is_err());
}

// ---- veil reversal ----

fn forward(p: u8, a: f64, c: u8) -> u8 {
    (c as f64 * a + p as f64 * (1.0 - a)).round() as u8
}

#[test]
fn alpha_zero_is_identity() {
    let rgb: Vec<u8> = (0..30).map(|i| (i * 8) as u8).collect();
    assert_eq!(unveil_rows(&rgb, 5, 2, 0.0, 0.0, [16, 16, 16]).unwrap(), rgb);
}

#[test]
fn a_forward_veiled_frame_is_restored_within_quantisation() {
    let (w, h) = (6usize, 4usize);
    let (a0, a1) = (0.753, 0.816);
    let orig: Vec<u8> = (0..w * h * 3).map(|i| ((i * 37) % 200 + 50) as u8).collect();
    let mut veiled = orig.clone();
    for y in 0..h {
        let a = a0 + (a1 - a0) * (y as f64 + 0.5) / h as f64;
        for x in 0..w {
            for k in 0..3 {
                let i = (y * w + x) * 3 + k;
                veiled[i] = forward(orig[i], a, 16);
            }
        }
    }
    let back = unveil_rows(&veiled, w, h, a0, a1, [16, 16, 16]).unwrap();
    let worst = back.iter().zip(&orig).map(|(b, o)| (*b as i32 - *o as i32).abs()).max().unwrap();
    // 8-bit rounding of the forward step is magnified by 1/(1-a) <= 5.5
    assert!(worst <= 6, "worst error {worst}");
}

#[test]
fn the_gradient_is_applied_per_row() {
    // same input pixel in the first and last row: different alpha -> different result
    let rgb = [100u8, 100, 100, 100, 100, 100, 100, 100, 100];
    let out = unveil_rows(&rgb, 1, 3, 0.2, 0.8, [0, 0, 0]).unwrap();
    assert!(out[0] < out[3] && out[3] < out[6], "{out:?}");
}

#[test]
fn unveil_clamps_to_0_255() {
    // pixel darker than the veil colour would need a negative value
    let out = unveil_rows(&[0u8, 0, 0], 1, 1, 0.5, 0.5, [100, 100, 100]).unwrap();
    assert_eq!(out, vec![0, 0, 0]);
    let out = unveil_rows(&[255u8, 255, 255], 1, 1, 0.9, 0.9, [0, 0, 0]).unwrap();
    assert_eq!(out, vec![255, 255, 255]);
}

#[test]
fn unveil_rejects_bad_alpha_and_sizes() {
    assert!(unveil_rows(&[0u8; 3], 1, 1, 1.0, 0.5, [0; 3]).is_err());
    assert!(unveil_rows(&[0u8; 3], 1, 1, 0.5, 1.0, [0; 3]).is_err());
    assert!(unveil_rows(&[0u8; 3], 1, 1, -0.1, 0.5, [0; 3]).is_err());
    assert!(unveil_rows(&[0u8; 3], 1, 1, f64::NAN, 0.5, [0; 3]).is_err());
    assert!(unveil_rows(&[0u8; 5], 1, 1, 0.5, 0.5, [0; 3]).is_err());
    assert!(unveil_rows(&[], 0, 0, 0.5, 0.5, [0; 3]).is_err());
}

#[test]
fn unveiling_a_veiled_fire_colour_makes_the_palette_match_again() {
    const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
    const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");
    let p = Palette::from_rgba(&[S0, S1], 24.0).unwrap();
    let fire: Vec<[u8; 3]> = S0.chunks(4).filter(|c| c[3] != 0).map(|c| [c[0], c[1], c[2]]).collect();
    let (w, h) = (40usize, 10usize);
    let mut veiled = Vec::new();
    for i in 0..w * h {
        let c = fire[(i * 13) % fire.len()];
        for k in 0..3 {
            veiled.push(forward(c[k], 0.785, 16));
        }
    }
    let before = p.count(&veiled, w, h).unwrap();
    let after = p.count(&unveil_rows(&veiled, w, h, 0.753, 0.816, [16, 16, 16]).unwrap(), w, h).unwrap();
    assert!(before < w * h / 4, "veiled fire should not match the palette: {before}");
    assert!(after as f64 > (w * h) as f64 * 0.8, "unveiled should: {after} of {}", w * h);
}

// ---- opening (outlier removal) ----

use fire_core::features::open_mask;

fn count(m: &[bool]) -> usize {
    m.iter().filter(|&&b| b).count()
}

#[test]
fn opening_with_radius_zero_is_identity() {
    let (m, w, h) = mask_from(&["#.#", ".#.", "..#"]);
    assert_eq!(open_mask(&m, w, h, 0).unwrap(), m);
}

#[test]
fn an_isolated_pixel_is_removed() {
    let (m, w, h) = mask_from(&[".....", "..#..", "....."]);
    assert_eq!(count(&open_mask(&m, w, h, 1).unwrap()), 0);
}

#[test]
fn a_3x3_block_survives_radius_1_exactly_and_a_2x2_block_does_not() {
    let (m, w, h) = mask_from(&["......", ".###..", ".###..", ".###..", "......"]);
    assert_eq!(open_mask(&m, w, h, 1).unwrap(), m);
    let (m2, w2, h2) = mask_from(&["....", ".##.", ".##.", "...."]);
    assert_eq!(count(&open_mask(&m2, w2, h2, 1).unwrap()), 0);
}

#[test]
fn radius_2_needs_a_5x5_block() {
    let mut rows: Vec<String> = vec![".......".into(); 7];
    for r in rows.iter_mut().take(6).skip(1) {
        *r = ".#####.".into();
    }
    // 5 rows (1..=5) of 5 -> a 5x5 block
    rows[6] = ".......".into();
    let refs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let (m, w, h) = mask_from(&refs);
    assert_eq!(count(&m), 25);
    assert_eq!(open_mask(&m, w, h, 2).unwrap(), m);
    // a 4x5 block does not survive
    let mut m2 = m.clone();
    for x in 0..w {
        m2[5 * w + x] = false;
    }
    assert_eq!(count(&open_mask(&m2, w, h, 2).unwrap()), 0);
}

#[test]
fn a_block_in_the_corner_is_not_eroded_by_the_border() {
    let (m, w, h) = mask_from(&["###...", "###...", "###...", "......"]);
    assert_eq!(open_mask(&m, w, h, 1).unwrap(), m);
}

#[test]
fn a_one_pixel_line_is_removed_but_a_thick_shape_with_a_spur_loses_only_the_spur() {
    let (m, w, h) = mask_from(&["......", "######", "......"]);
    assert_eq!(count(&open_mask(&m, w, h, 1).unwrap()), 0);
    let (m, w, h) = mask_from(&["..........", ".###......", ".######...", ".###......", ".........."]);
    let o = open_mask(&m, w, h, 1).unwrap();
    // the 3x3 body stays, the 3-pixel spur (cols 4..6 of the middle row) goes
    assert_eq!(count(&o), 9);
}

#[test]
fn two_blobs_joined_by_a_bridge_are_separated() {
    let (m, w, h) = mask_from(&["###...###", "#########", "###...###"]);
    let o = open_mask(&m, w, h, 1).unwrap();
    assert_eq!(count(&o), 18);
    assert_eq!(blob_stats(&o, w, h, 1).unwrap().largest, 9);
    assert_eq!(blob_stats(&m, w, h, 1).unwrap().largest, 21); // joined before opening
}

#[test]
fn opening_empty_and_full_masks() {
    assert_eq!(count(&open_mask(&vec![false; 20], 5, 4, 1).unwrap()), 0);
    assert_eq!(count(&open_mask(&vec![true; 20], 5, 4, 1).unwrap()), 20);
    // a radius bigger than the mask: a full mask is still full (nothing but the border is in the way)
    assert_eq!(count(&open_mask(&vec![true; 6], 3, 2, 5).unwrap()), 6);
}

#[test]
fn opening_input_errors() {
    assert!(open_mask(&[true; 5], 2, 2, 1).is_err());
    assert!(open_mask(&[], 0, 0, 1).is_err());
}

#[test]
fn opening_never_adds_pixels() {
    let mut seed = 5u64;
    let (w, h) = (40usize, 30usize);
    let m: Vec<bool> = (0..w * h)
        .map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % 3 != 0
        })
        .collect();
    for r in 1..=2 {
        let o = open_mask(&m, w, h, r).unwrap();
        assert!(o.iter().zip(&m).all(|(a, b)| !*a || *b), "r={r}");
        // opening is idempotent
        assert_eq!(open_mask(&o, w, h, r).unwrap(), o, "r={r}");
    }
}

#[test]
fn opening_a_full_hd_mask_is_fast() {
    let (w, h) = (1920usize, 1080usize);
    let m: Vec<bool> = (0..w * h).map(|i| (i / w + i % w) % 5 != 0).collect();
    let t = std::time::Instant::now();
    let _ = open_mask(&m, w, h, 2).unwrap();
    assert!(t.elapsed().as_secs_f64() < 1.5, "{:?}", t.elapsed());
}

// ---- blob labelling / shape ----

use fire_core::features::label_blobs;

#[test]
fn a_single_pixel_blob() {
    let (m, w, h) = mask_from(&["...", ".#.", "..."]);
    let (labels, infos) = label_blobs(&m, w, h).unwrap();
    assert_eq!(infos.len(), 1);
    let b = infos[0];
    assert_eq!((b.area, b.perimeter, b.x0, b.y0, b.x1, b.y1), (1, 4, 1, 1, 2, 2));
    assert_eq!(labels[4], 1);
    assert_eq!(labels.iter().filter(|&&l| l == 0).count(), 8);
}

#[test]
fn perimeters_of_squares() {
    let (m, w, h) = mask_from(&["....", ".##.", ".##.", "...."]);
    assert_eq!(label_blobs(&m, w, h).unwrap().1[0].perimeter, 8);
    let (m, w, h) = mask_from(&[".....", ".###.", ".###.", ".###.", "....."]);
    assert_eq!(label_blobs(&m, w, h).unwrap().1[0].perimeter, 12);
}

#[test]
fn a_hole_adds_its_own_edge() {
    let (m, w, h) = mask_from(&[".....", ".###.", ".#.#.", ".###.", "....."]);
    let b = label_blobs(&m, w, h).unwrap().1[0];
    assert_eq!((b.area, b.perimeter), (8, 12 + 4));
}

#[test]
fn an_l_shape_perimeter() {
    // only sides facing an in-frame non-blob pixel count: (0,0) right, (0,1) right, (1,2) up, (2,2) up
    let (m, w, h) = mask_from(&["#..", "#..", "###"]);
    let b = label_blobs(&m, w, h).unwrap().1[0];
    assert_eq!((b.area, b.perimeter), (5, 4));
}

#[test]
fn sides_on_the_frame_edge_do_not_count() {
    let (m, w, h) = mask_from(&["#.", ".."]);
    assert_eq!(label_blobs(&m, w, h).unwrap().1[0].perimeter, 2); // only right and bottom neighbours are in the frame
    let (m, w, h) = mask_from(&["##", "##"]);
    assert_eq!(label_blobs(&m, w, h).unwrap().1[0].perimeter, 0); // the whole frame
}

#[test]
fn diagonal_pixels_get_separate_labels_in_scan_order() {
    let (m, w, h) = mask_from(&["#..", ".#.", "..#"]);
    let (labels, infos) = label_blobs(&m, w, h).unwrap();
    assert_eq!(infos.len(), 3);
    assert_eq!((labels[0], labels[4], labels[8]), (1, 2, 3));
}

#[test]
fn bbox_and_areas_are_exact_and_cover_the_mask() {
    let (m, w, h) = mask_from(&["##....", ".#..#.", ".#..##", "......"]);
    let (labels, infos) = label_blobs(&m, w, h).unwrap();
    assert_eq!(infos.len(), 2);
    assert_eq!((infos[0].area, infos[0].x0, infos[0].y0, infos[0].x1, infos[0].y1), (4, 0, 0, 2, 3));
    assert_eq!((infos[1].area, infos[1].x0, infos[1].y0, infos[1].x1, infos[1].y1), (3, 4, 1, 6, 3));
    assert_eq!(infos.iter().map(|b| b.area).sum::<usize>(), m.iter().filter(|&&b| b).count());
    assert!(labels.iter().zip(&m).all(|(l, b)| (*l > 0) == *b));
}

#[test]
fn label_blobs_errors_and_empty() {
    assert!(label_blobs(&[true; 5], 2, 2).is_err());
    assert!(label_blobs(&[], 0, 0).is_err());
    let (l, i) = label_blobs(&[false; 6], 3, 2).unwrap();
    assert!(i.is_empty() && l.iter().all(|&x| x == 0));
}

#[test]
fn label_blobs_full_hd_is_fast() {
    let (w, h) = (1920usize, 1080usize);
    let m: Vec<bool> = (0..w * h).map(|i| (i / w) % 3 != 0).collect();
    let t = std::time::Instant::now();
    let (_, infos) = label_blobs(&m, w, h).unwrap();
    assert!(!infos.is_empty());
    assert!(t.elapsed().as_secs_f64() < 1.5, "{:?}", t.elapsed());
}
