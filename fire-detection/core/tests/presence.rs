use fire_core::presence::Palette;

const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn palette(d: f64) -> Palette {
    Palette::from_rgba(&[S0, S1], d).unwrap()
}

fn rnd(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u32
}

fn solid(w: usize, h: usize, c: [u8; 3]) -> Vec<u8> {
    c.iter().copied().cycle().take(w * h * 3).collect()
}

/// Every opaque pixel of both sheets as an RGB triple.
fn opaque_colours() -> Vec<[u8; 3]> {
    let mut v = Vec::new();
    for s in [S0, S1] {
        for p in s.chunks(4) {
            if p[3] != 0 {
                v.push([p[0], p[1], p[2]]);
            }
        }
    }
    v
}

#[test]
fn a_frame_of_a_fire_colour_is_all_fire() {
    let p = palette(24.0);
    let c = opaque_colours()[1234];
    let f = solid(40, 30, c);
    assert_eq!(p.count(&f, 40, 30).unwrap(), 40 * 30);
}

#[test]
fn every_palette_colour_counts() {
    let p = palette(0.0);
    for c in opaque_colours() {
        assert_eq!(p.count(&solid(2, 2, c), 2, 2).unwrap(), 4, "{c:?}");
    }
}

#[test]
fn neutral_and_cold_colours_do_not_count() {
    let p = palette(24.0);
    for c in [
        [0, 0, 0],
        [90, 90, 90],
        [128, 128, 128],
        [20, 60, 200],
        [40, 160, 50],
        [110, 20, 20],
    ] {
        assert_eq!(p.count(&solid(10, 10, c), 10, 10).unwrap(), 0, "{c:?}");
    }
}

#[test]
fn grey_noise_background_has_no_fire() {
    let p = palette(24.0);
    let (w, h) = (100usize, 80usize);
    let mut seed = 7u64;
    let mut f = vec![0u8; w * h * 3];
    for px in f.chunks_mut(3) {
        let v = 30 + (rnd(&mut seed) % 90) as u8;
        px.copy_from_slice(&[v, v, v]);
    }
    assert_eq!(p.count(&f, w, h).unwrap(), 0);
}

#[test]
fn counts_only_the_fire_pixels_of_a_mixed_frame() {
    let p = palette(24.0);
    let (w, h) = (50usize, 40usize);
    let mut f = solid(w, h, [60, 60, 60]);
    let fire = opaque_colours()[500];
    for i in 0..300 {
        f[i * 3..i * 3 + 3].copy_from_slice(&fire);
    }
    assert_eq!(p.count(&f, w, h).unwrap(), 300);
}

#[test]
fn small_compression_style_shifts_still_count() {
    let p = palette(24.0);
    let mut seed = 3u64;
    let mut hit = 0;
    let cols = opaque_colours();
    let n = 2000;
    for i in 0..n {
        let c = cols[(i * 7) % cols.len()];
        let shifted: Vec<u8> = c
            .iter()
            .map(|&v| (v as i32 + (rnd(&mut seed) % 17) as i32 - 8).clamp(0, 255) as u8)
            .collect();
        hit += p.count(&shifted, 1, 1).unwrap();
    }
    assert!(hit as f64 >= n as f64 * 0.99, "{hit} of {n}");
}

#[test]
fn the_distance_edge_is_exact() {
    // a palette of the single colour (200,100,50): (200,100,50-d) is at distance d
    let mut sheet = vec![0u8; 16 * 16 * 4];
    sheet[..4].copy_from_slice(&[200, 100, 50, 255]);
    let p = Palette::from_rgba(&[&sheet], 10.0).unwrap();
    assert_eq!(p.count(&[200, 100, 41], 1, 1).unwrap(), 1); // 9 away
    assert_eq!(p.count(&[200, 100, 40], 1, 1).unwrap(), 1); // 10 away: inside
    assert_eq!(p.count(&[200, 100, 39], 1, 1).unwrap(), 0); // 11 away
    assert_eq!(p.count(&[206, 108, 50], 1, 1).unwrap(), 1); // 6,8 -> exactly 10 away diagonally
    assert_eq!(p.count(&[207, 108, 50], 1, 1).unwrap(), 0); // 7,8 -> ~10.6
}

#[test]
fn a_bigger_tolerance_never_counts_less() {
    let small = palette(8.0);
    let big = palette(30.0);
    let mut seed = 9u64;
    let (w, h) = (60usize, 60usize);
    let mut f = vec![0u8; w * h * 3];
    for b in f.iter_mut() {
        *b = (rnd(&mut seed) % 256) as u8;
    }
    assert!(big.count(&f, w, h).unwrap() >= small.count(&f, w, h).unwrap());
}

#[test]
fn rows_can_be_skipped_for_the_sync_strip() {
    let p = palette(24.0);
    let (w, h) = (20usize, 10usize);
    let mut f = solid(w, h, [0, 0, 0]);
    let fire = opaque_colours()[10];
    for i in 0..w * 2 {
        f[i * 3..i * 3 + 3].copy_from_slice(&fire); // first two rows
    }
    assert_eq!(p.count(&f, w, h).unwrap(), 40);
    assert_eq!(p.count_rows(&f, w, h, 2, h).unwrap(), 0);
    assert_eq!(p.count_rows(&f, w, h, 0, 1).unwrap(), 20);
}

#[test]
fn bad_input_is_an_error() {
    let p = palette(24.0);
    assert!(p.count(&[0u8; 5], 2, 1).is_err());
    assert!(p.count(&[], 0, 0).is_err());
    assert!(p.count(&[0u8; 6], 0, 2).is_err());
    assert!(p.count_rows(&[0u8; 12], 2, 2, 1, 3).is_err()); // rows past the end
    assert!(p.count_rows(&[0u8; 12], 2, 2, 2, 1).is_err()); // reversed
    assert!(Palette::from_rgba(&[], 10.0).is_err());
    assert!(Palette::from_rgba(&[&S0[..10]], 10.0).is_err());
    assert!(Palette::from_rgba(&[S0], -1.0).is_err());
    assert!(Palette::from_rgba(&[S0], f64::NAN).is_err());
}

#[test]
fn a_fully_transparent_sheet_is_rejected() {
    let blank = vec![0u8; 16 * 16 * 4];
    assert!(Palette::from_rgba(&[&blank], 10.0).is_err());
}

#[test]
fn the_mask_marks_exactly_the_counted_pixels() {
    let p = palette(24.0);
    let fire = opaque_colours()[77];
    let mut f = solid(8, 4, [60, 60, 60]);
    for i in [0usize, 5, 17, 31] {
        f[i * 3..i * 3 + 3].copy_from_slice(&fire);
    }
    let m = p.mask(&f, 8, 4, 0, 4).unwrap();
    assert_eq!(m.len(), 32);
    assert_eq!(m.iter().filter(|&&b| b).count(), p.count(&f, 8, 4).unwrap());
    assert!(m[0] && m[5] && m[17] && m[31] && !m[1]);
    // skipping rows shortens the mask
    assert_eq!(p.mask(&f, 8, 4, 2, 4).unwrap().len(), 16);
    assert!(p.mask(&f, 8, 4, 3, 2).is_err());
}
