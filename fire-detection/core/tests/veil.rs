//! Detecting the vanilla GUI background veil (pause menu, inventory, ...) from pixels alone.
//! The game draws fillGradient(0xC0101010 -> 0xD0101010): colour (16,16,16), alpha 192/255 at the top to 208/255 at the bottom.
//! So every veiled pixel is >= 16*alpha (about 12) and <= 16*alpha + 255*(1 - alpha) (about 76 at the top, 60 at the bottom).

use fire_core::veil::{detect_veil, A_BOTTOM, A_TOP};

const W: usize = 320;
const H: usize = 180;

fn rnd(seed: &mut u64) -> u32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 33) as u32
}

/// A natural-ish scene: smooth gradients plus noise, values spread over 0..255.
fn scene(seed: u64, lo: u8, hi: u8) -> Vec<u8> {
    let mut s = seed;
    let mut f = vec![0u8; W * H * 3];
    for y in 0..H {
        for x in 0..W {
            let base = lo as f64 + (hi - lo) as f64 * (0.5 + 0.5 * ((x as f64 / 23.0).sin() * (y as f64 / 17.0).cos()));
            for k in 0..3 {
                let v = base + (rnd(&mut s) % 21) as f64 - 10.0 + k as f64 * 4.0;
                f[(y * W + x) * 3 + k] = v.clamp(lo as f64, hi as f64) as u8;
            }
        }
    }
    f
}

/// The game's forward veil: out = c * a + p * (1 - a), a growing from the top row to the bottom row.
fn veil(frame: &[u8]) -> Vec<u8> {
    let mut out = frame.to_vec();
    for y in 0..H {
        let a = A_TOP + (A_BOTTOM - A_TOP) * (y as f64 + 0.5) / H as f64;
        for x in 0..W {
            for k in 0..3 {
                let i = (y * W + x) * 3 + k;
                out[i] = (16.0 * a + frame[i] as f64 * (1.0 - a)).round() as u8;
            }
        }
    }
    out
}

// ---- basics ----

#[test]
fn a_veiled_scene_is_detected() {
    for seed in 0..5 {
        let f = veil(&scene(seed, 0, 255));
        assert!(detect_veil(&f, W, H, 0).unwrap(), "seed {seed}");
    }
}

#[test]
fn a_normal_bright_scene_is_not_veiled() {
    for seed in 0..5 {
        let f = scene(100 + seed, 0, 255);
        assert!(!detect_veil(&f, W, H, 0).unwrap(), "seed {seed}");
    }
}

#[test]
fn a_dark_scene_with_black_pixels_is_not_veiled() {
    // a dark Nether cave: nothing above 70, but real blacks, which a veil can never produce
    for seed in 0..5 {
        let f = scene(200 + seed, 0, 70);
        assert!(!detect_veil(&f, W, H, 0).unwrap(), "seed {seed}");
    }
}

#[test]
fn a_veiled_totally_black_scene_is_detected() {
    // black through the veil = (16 * a): every pixel at the floor
    let f = veil(&vec![0u8; W * H * 3]);
    assert!(detect_veil(&f, W, H, 0).unwrap());
}

#[test]
fn a_plain_black_frame_is_not_veiled() {
    assert!(!detect_veil(&vec![0u8; W * H * 3], W, H, 0).unwrap());
}

// ---- GUI widgets over the veil ----

#[test]
fn bright_buttons_and_text_over_the_veil_do_not_hide_it() {
    let mut f = veil(&scene(7, 0, 255));
    // a pause-menu-like block of bright widgets (about 6% of the frame)
    for y in 60..90 {
        for x in 100..220 {
            for k in 0..3 {
                f[(y * W + x) * 3 + k] = 200;
            }
        }
    }
    assert!(detect_veil(&f, W, H, 0).unwrap());
}

#[test]
fn a_big_bright_area_means_it_is_not_a_veil() {
    let mut f = veil(&scene(8, 0, 255));
    for y in 0..H / 2 {
        for x in 0..W {
            for k in 0..3 {
                f[(y * W + x) * 3 + k] = 220; // half the frame is bright: no veil can do that
            }
        }
    }
    assert!(!detect_veil(&f, W, H, 0).unwrap());
}

// ---- compression noise, strip rows ----

#[test]
fn small_compression_noise_keeps_the_detection() {
    let mut s = 3u64;
    let mut f = veil(&scene(9, 0, 255));
    for b in f.iter_mut() {
        *b = (*b as i32 + (rnd(&mut s) % 5) as i32 - 2).clamp(0, 255) as u8;
    }
    assert!(detect_veil(&f, W, H, 0).unwrap());
}

#[test]
fn the_sync_strip_rows_are_ignored() {
    let mut f = veil(&scene(10, 0, 255));
    // a 24-row strip of black and white cells is 13% of this frame: too many out-of-range pixels unless it is skipped
    for y in 0..24 {
        for x in 0..W {
            let v = if (x / 4) % 2 == 0 { 255 } else { 0 };
            for k in 0..3 {
                f[(y * W + x) * 3 + k] = v;
            }
        }
    }
    assert!(!detect_veil(&f, W, H, 0).unwrap(), "premise: 13% bad pixels break the veil test");
    assert!(detect_veil(&f, W, H, 24).unwrap(), "with y0 = 24 the strip is skipped");
}

// ---- errors ----

#[test]
fn bad_input_is_an_error() {
    assert!(detect_veil(&[0u8; 5], 2, 1, 0).is_err());
    assert!(detect_veil(&[], 0, 0, 0).is_err());
    assert!(detect_veil(&vec![0u8; W * H * 3], W, H, H).is_err()); // no rows left
    assert!(detect_veil(&vec![0u8; W * H * 3], W, H, H + 1).is_err());
}

#[test]
fn a_tiny_frame_does_not_panic() {
    let _ = detect_veil(&[20u8; 3 * 4 * 4], 4, 4, 0);
}
