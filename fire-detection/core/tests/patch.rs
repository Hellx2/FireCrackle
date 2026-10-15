use fire_core::patch::{
    frame_patch_features, patch_truth, pool_patches, PatchGrid, PatchModel, PatchTruth, NFEAT,
    PATCH,
};
use fire_core::presence::Palette;

const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn pals() -> (Palette, Palette) {
    (
        Palette::from_rgba(&[S0, S1], 24.0).unwrap(),
        Palette::from_rgba(&[S0, S1], 8.0).unwrap(),
    )
}

fn solid(w: usize, h: usize, c: [u8; 3]) -> Vec<u8> {
    c.iter().copied().cycle().take(w * h * 3).collect()
}

fn feats(frame: &[u8], w: usize, h: usize) -> PatchGrid {
    let (loose, strict) = pals();
    frame_patch_features(frame, w, h, 0, &loose, &strict).unwrap()
}

fn idx(name: &str) -> usize {
    fire_core::patch::FEATURE_NAMES
        .iter()
        .position(|n| *n == name)
        .unwrap_or_else(|| panic!("no feature {name}"))
}

fn fire_colour() -> [u8; 3] {
    let p = S0.chunks(4).find(|c| c[3] != 0).unwrap();
    [p[0], p[1], p[2]]
}

// ---- grid ----

#[test]
fn the_grid_drops_partial_patches() {
    let g = feats(&solid(40, 35, [0, 0, 0]), 40, 35);
    assert_eq!((g.gx, g.gy), (2, 2)); // 40/16 = 2, 35/16 = 2
    assert_eq!(g.feats.len(), 4);
    assert_eq!(NFEAT, fire_core::patch::FEATURE_NAMES.len());
}

#[test]
fn rows_above_y0_are_skipped_and_too_small_frames_are_errors() {
    let (loose, strict) = pals();
    let f = solid(32, 48, [0, 0, 0]);
    let g = frame_patch_features(&f, 32, 48, 16, &loose, &strict).unwrap();
    assert_eq!((g.gx, g.gy), (2, 2)); // (48-16)/16
    assert!(frame_patch_features(&f, 32, 48, 40, &loose, &strict).is_err()); // less than one patch row left
    assert!(frame_patch_features(&f, 31, 48, 0, &loose, &strict).is_err()); // wrong buffer size
    assert!(frame_patch_features(&solid(8, 8, [0; 3]), 8, 8, 0, &loose, &strict).is_err()); // smaller than a patch
    assert!(frame_patch_features(&f, 32, 48, 49, &loose, &strict).is_err()); // y0 past the end
}

// ---- values ----

#[test]
fn a_dark_flat_patch() {
    let g = feats(&solid(16, 16, [0, 0, 0]), 16, 16);
    let f = g.feats[0];
    assert_eq!(f[idx("pal")], 0.0);
    assert_eq!(f[idx("strict")], 0.0);
    assert_eq!(f[idx("dark")], 1.0);
    assert_eq!(f[idx("white")], 0.0);
    assert_eq!(f[idx("lum")], 0.0);
    assert_eq!(f[idx("lumstd")], 0.0);
    assert_eq!(f[idx("grad")], 0.0);
    assert_eq!(f[idx("big")], 0.0);
    assert_eq!(f[idx("sat")], 0.0);
    assert_eq!(f[idx("warm")], 0.0);
}

#[test]
fn a_flat_fire_colour_patch() {
    let c = fire_colour();
    let f = feats(&solid(16, 16, c), 16, 16).feats[0];
    assert_eq!(f[idx("pal")], 1.0);
    assert_eq!(f[idx("strict")], 1.0);
    assert_eq!(f[idx("grad")], 0.0);
    let l = (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32) / 255.0;
    assert!((f[idx("lum")] - l).abs() < 1e-3);
    let warm = (c[0] as f32 - c[2] as f32) / 255.0;
    assert!(
        (f[idx("warm")] - warm).abs() < 1e-3,
        "{} vs {warm}",
        f[idx("warm")]
    );
}

#[test]
fn white_and_bright_shares() {
    let mut fr = solid(16, 16, [0, 0, 0]);
    for i in 0..64 {
        fr[i * 3..i * 3 + 3].copy_from_slice(&[255, 255, 255]); // a quarter of the patch
    }
    let f = feats(&fr, 16, 16).feats[0];
    assert!((f[idx("white")] - 0.25).abs() < 1e-6);
    assert!((f[idx("dark")] - 0.75).abs() < 1e-6);
    assert!(
        (f[idx("pal")] - 0.25).abs() < 1e-6,
        "pure white is in the fire palette: {}",
        f[idx("pal")]
    );
    assert!((f[idx("bright")] - 0.25).abs() < 1e-6);
}

#[test]
fn a_checkerboard_has_maximal_texture() {
    let mut fr = vec![0u8; 16 * 16 * 3];
    for y in 0..16 {
        for x in 0..16 {
            let v = if (x + y) % 2 == 0 { 0 } else { 200 };
            fr[(y * 16 + x) * 3..(y * 16 + x) * 3 + 3].copy_from_slice(&[v, v, v]);
        }
    }
    let f = feats(&fr, 16, 16).feats[0];
    assert!(
        (f[idx("grad")] - 400.0 / 255.0).abs() < 0.01,
        "{}",
        f[idx("grad")]
    );
    assert_eq!(f[idx("big")], 1.0);
    assert!(f[idx("lumstd")] > 0.3);
}

#[test]
fn saturation_of_grey_is_zero_and_of_pure_red_is_one() {
    assert_eq!(
        feats(&solid(16, 16, [120, 120, 120]), 16, 16).feats[0][idx("sat")],
        0.0
    );
    assert!((feats(&solid(16, 16, [200, 0, 0]), 16, 16).feats[0][idx("sat")] - 1.0).abs() < 1e-6);
}

#[test]
fn ring_is_the_mean_palette_share_of_existing_neighbours() {
    // 3x3 patches: centre is fire colour, everything else dark. centre.ring = 0; a corner patch has 3 neighbours.
    let c = fire_colour();
    let (w, h) = (48usize, 48usize);
    let mut fr = solid(w, h, [0, 0, 0]);
    for y in 16..32 {
        for x in 16..32 {
            fr[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&c);
        }
    }
    let g = feats(&fr, w, h);
    let ring = |px: usize, py: usize| g.feats[py * g.gx + px][idx("ring")];
    assert_eq!(ring(1, 1), 0.0); // 8 dark neighbours
    assert!((ring(0, 0) - 1.0 / 3.0).abs() < 1e-6); // neighbours: (1,0) (0,1) (1,1): one of three is fire
    assert!((ring(1, 0) - 1.0 / 5.0).abs() < 1e-6); // 5 neighbours, one fire
    assert!((ring(0, 1) - 1.0 / 5.0).abs() < 1e-6);
}

#[test]
fn one_patch_frame_has_ring_zero_not_nan() {
    let g = feats(&solid(16, 16, fire_colour()), 16, 16);
    assert_eq!(g.feats[0][idx("ring")], 0.0);
    assert!(g.feats[0].iter().all(|v| !v.is_nan()));
}

// ---- truth ----

const B: [f64; 4] = [100.0, 100.0, 200.0, 200.0];

#[test]
fn a_patch_inside_a_visible_box_is_positive() {
    assert_eq!(patch_truth(&[(B, 0.9)], 112, 112), PatchTruth::Pos);
}

#[test]
fn a_patch_far_from_every_box_is_negative() {
    assert_eq!(patch_truth(&[(B, 0.9)], 400, 400), PatchTruth::Neg);
    assert_eq!(patch_truth(&[], 0, 0), PatchTruth::Neg);
}

#[test]
fn the_patch_coverage_edge_is_inclusive_at_half() {
    // patch x 92..108 covers 100..108 of the box: exactly 8 of 16 columns = 50%
    assert_eq!(patch_truth(&[(B, 0.9)], 92, 120), PatchTruth::Pos);
    // patch x 91..107 covers 7 of 16 columns
    assert_eq!(patch_truth(&[(B, 0.9)], 91, 120), PatchTruth::Skip);
}

#[test]
fn the_negative_margin_is_eight_pixels() {
    // patch x 76..92 ends 8 px left of the box edge at 100: exactly 8 away -> still negative
    assert_eq!(patch_truth(&[(B, 0.9)], 76, 120), PatchTruth::Neg);
    assert_eq!(patch_truth(&[(B, 0.9)], 77, 120), PatchTruth::Skip);
}

#[test]
fn a_low_visibility_box_never_makes_a_positive_but_blocks_negatives() {
    assert_eq!(patch_truth(&[(B, 0.29)], 112, 112), PatchTruth::Skip);
    assert_eq!(patch_truth(&[(B, 0.30)], 112, 112), PatchTruth::Pos);
    assert_eq!(patch_truth(&[(B, 0.29)], 400, 400), PatchTruth::Neg);
}

#[test]
fn any_box_can_make_a_positive() {
    let boxes = [([0.0, 0.0, 10.0, 10.0], 0.9), (B, 0.9)];
    assert_eq!(patch_truth(&boxes, 112, 112), PatchTruth::Pos);
}

#[test]
fn degenerate_boxes_do_not_panic() {
    let boxes = [
        ([50.0, 50.0, 50.0, 50.0], 1.0),
        ([f64::NAN, 0.0, 1.0, 1.0], 1.0),
    ];
    let _ = patch_truth(&boxes, 48, 48);
    assert_eq!(patch_truth(&boxes, 900, 900), PatchTruth::Neg);
}

// ---- model ----

const TREE: &str = r#"{"nodes":[
  {"f":0,"t":0.5,"l":1,"r":2},
  {"leaf":0.1},
  {"f":3,"t":0.2,"l":3,"r":4},
  {"leaf":0.4},
  {"leaf":0.9}
]}"#;

fn feat(vals: &[(usize, f32)]) -> [f32; NFEAT] {
    let mut f = [0.0f32; NFEAT];
    for (i, v) in vals {
        f[*i] = *v;
    }
    f
}

#[test]
fn the_tree_follows_thresholds_with_less_or_equal_going_left() {
    let m = PatchModel::from_json(TREE).unwrap();
    assert_eq!(m.predict(&feat(&[(0, 0.5)])), 0.1); // 0.5 <= 0.5 -> left
    assert_eq!(m.predict(&feat(&[(0, 0.6), (3, 0.2)])), 0.4);
    assert_eq!(m.predict(&feat(&[(0, 0.6), (3, 0.21)])), 0.9);
}

#[test]
fn malformed_models_are_errors() {
    for bad in [
        "",
        "{}",
        r#"{"nodes":[]}"#,
        r#"{"nodes":[{"f":0,"t":0.5,"l":1,"r":2}]}"#, // children out of range
        r#"{"nodes":[{"f":0,"t":0.5,"l":0,"r":0}]}"#, // cycle
        r#"{"nodes":[{"f":99,"t":0.5,"l":1,"r":2},{"leaf":0},{"leaf":1}]}"#, // feature out of range
        r#"{"nodes":[{"f":0,"l":1,"r":2},{"leaf":0},{"leaf":1}]}"#, // no threshold
        r#"{"nodes":[{"leaf":"x"}]}"#,
    ] {
        assert!(PatchModel::from_json(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_single_leaf_model_is_valid() {
    let m = PatchModel::from_json(r#"{"nodes":[{"leaf":0.7}]}"#).unwrap();
    assert_eq!(m.predict(&[0.0; NFEAT]), 0.7);
}

// ---- pooling ----

fn grid_of(rows: &[&str]) -> (Vec<bool>, usize, usize) {
    let h = rows.len();
    let w = rows[0].len();
    (
        rows.iter()
            .flat_map(|r| r.bytes().map(|c| c == b'#'))
            .collect(),
        w,
        h,
    )
}

#[test]
fn pooling_counts_positives_and_the_largest_cluster() {
    let (m, w, h) = grid_of(&["##..#", "##...", ".....", "#...#"]);
    let p = pool_patches(&m, w, h).unwrap();
    assert_eq!(p.positives, 7); // 3 + 2 + 0 + 2 by hand
    assert_eq!(p.largest_cluster, 4);
}

#[test]
fn diagonal_patches_are_separate_clusters_and_empty_is_zero() {
    let (m, w, h) = grid_of(&["#.", ".#"]);
    let p = pool_patches(&m, w, h).unwrap();
    assert_eq!((p.positives, p.largest_cluster), (2, 1));
    let p = pool_patches(&[false; 6], 3, 2).unwrap();
    assert_eq!((p.positives, p.largest_cluster), (0, 0));
    assert!(pool_patches(&[true; 5], 3, 2).is_err());
}

#[test]
fn patch_is_16() {
    assert_eq!(PATCH, 16);
}
