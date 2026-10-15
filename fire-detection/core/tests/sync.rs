use fire_core::sync::{
    cell_size, crc8, decode, stamp, strip_size, SyncCode, SyncError, CELLS, MAX_ID, MAX_TICK,
};

// ---------- helpers ----------

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }
}

fn height_for(width: usize) -> usize {
    width * 9 / 16
}

fn noisy(width: usize, height: usize, seed: u64) -> Vec<u8> {
    let mut rng = Lcg(seed);
    (0..width * height).map(|_| rng.byte()).collect()
}

fn code(tick: u32, frame_id: u32) -> SyncCode {
    SyncCode { tick, frame_id }
}

fn stamped(width: usize, height: usize, tick: u32, id: u32, bg: u8) -> Vec<u8> {
    let mut f = vec![bg; width * height];
    stamp(&mut f, width, height, tick, id).unwrap();
    f
}

fn resize_nearest(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut out = vec![0u8; dw * dh];
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            out[y * dw + x] = src[sy * sw + x * sw / dw];
        }
    }
    out
}

fn blur_h(src: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let lo = x.saturating_sub(radius);
            let hi = (x + radius).min(w - 1);
            let sum: u32 = (lo..=hi).map(|i| src[y * w + i] as u32).sum();
            out[y * w + x] = (sum / (hi - lo + 1) as u32) as u8;
        }
    }
    out
}

/// Pixel x-range of strip cell `i` at the given width (same rule the plan gives encoders).
fn cell_range(width: usize, i: usize) -> (usize, usize) {
    let c = cell_size(width);
    (
        (i as f64 * c).round() as usize,
        ((i + 1) as f64 * c).round() as usize,
    )
}

// ---------- basics ----------

#[test]
fn crc8_matches_standard_check_value() {
    assert_eq!(crc8(b"123456789"), 0xF4);
    assert_eq!(crc8(&[]), 0x00);
}

#[test]
fn cell_size_is_8_at_1080p_and_scales_with_width() {
    assert_eq!(cell_size(1920), 8.0);
    assert_eq!(cell_size(3840), 16.0);
    assert!((cell_size(1280) - 16.0 / 3.0).abs() < 1e-9);
}

#[test]
fn strip_size_geometry() {
    assert_eq!(CELLS, 58);
    assert_eq!(MAX_TICK, 0xFF_FFFF);
    assert_eq!(MAX_ID, 0xFF_FFFF);
    assert_eq!(strip_size(1920), Some((464, 8)));
    assert_eq!(strip_size(3840), Some((928, 16)));
    assert_eq!(strip_size(1280), Some((309, 5)));
}

#[test]
fn roundtrip_edge_pairs_at_1080p() {
    let (w, h) = (1920, 1080);
    let pairs = [
        (0, 0),
        (1, 0),
        (0, 1),
        (MAX_TICK, 0),
        (0, MAX_ID),
        (MAX_TICK, MAX_ID),
        (0x123456, 0xABCDEF),
        (0xABCDEF, 0x123456),
        (0x800000, 0x000001),
        (0x555555, 0xAAAAAA),
    ];
    for (tick, id) in pairs {
        let f = stamped(w, h, tick, id, 0);
        assert_eq!(decode(&f, w, h), Some(code(tick, id)), "tick {tick:#x} id {id:#x}");
    }
}

#[test]
fn tick_and_frame_id_come_back_in_their_own_slots() {
    let (w, h) = (1920, 1080);
    let a = decode(&stamped(w, h, 0x000010, 0x000020, 0), w, h).unwrap();
    let b = decode(&stamped(w, h, 0x000020, 0x000010, 0), w, h).unwrap();
    assert_eq!((a.tick, a.frame_id), (0x10, 0x20));
    assert_eq!((b.tick, b.frame_id), (0x20, 0x10));
}

#[test]
fn many_ticks_share_one_frame_id_space_independently() {
    // Several rendered frames in one tick: same tick, different ids all decode distinctly.
    let (w, h) = (1920, 1080);
    for id in 100..110 {
        assert_eq!(decode(&stamped(w, h, 4242, id, 40), w, h), Some(code(4242, id)));
    }
}

#[test]
fn roundtrip_many_random_pairs() {
    let (w, h) = (1920, 1080);
    let mut rng = Lcg(7);
    for _ in 0..200 {
        let tick = rng.next() & MAX_TICK;
        let id = rng.next() & MAX_ID;
        let f = stamped(w, h, tick, id, 90);
        assert_eq!(decode(&f, w, h), Some(code(tick, id)), "tick {tick:#x} id {id:#x}");
    }
}

#[test]
fn roundtrip_at_common_resolutions() {
    for w in [854, 1280, 1920, 2560, 3840] {
        let h = height_for(w);
        for (tick, id) in [(0, 0), (0xABCDE, 0x54321), (MAX_TICK, MAX_ID)] {
            let f = stamped(w, h, tick, id, 30);
            assert_eq!(decode(&f, w, h), Some(code(tick, id)), "{w}px {tick:#x}/{id:#x}");
        }
    }
}

#[test]
fn stamp_only_touches_the_strip() {
    let (w, h) = (1920, 1080);
    let bg = noisy(w, h, 1);
    let mut f = bg.clone();
    stamp(&mut f, w, h, 0x42, 0x77).unwrap();
    let (sw, sh) = strip_size(w).unwrap();
    for y in 0..h {
        for x in 0..w {
            if x >= sw || y >= sh {
                assert_eq!(f[y * w + x], bg[y * w + x], "pixel ({x},{y}) changed");
            }
        }
    }
}

#[test]
fn stamp_is_opaque_over_a_noisy_background() {
    let (w, h) = (1920, 1080);
    let mut f = noisy(w, h, 99);
    stamp(&mut f, w, h, 0xC0FFEE, 0x0DDBA1).unwrap();
    assert_eq!(decode(&f, w, h), Some(code(0xC0FFEE, 0x0DDBA1)));
}

// ---------- survives what video does to it ----------

#[test]
fn survives_rescaling_after_stamping() {
    let (w, h) = (1920, 1080);
    let f = stamped(w, h, 0x13579B, 0x2468AC, 40);
    for (dw, dh) in [(1280, 720), (960, 540), (2560, 1440), (854, 480)] {
        let r = resize_nearest(&f, w, h, dw, dh);
        assert_eq!(decode(&r, dw, dh), Some(code(0x13579B, 0x2468AC)), "{dw}x{dh}");
    }
}

#[test]
fn survives_horizontal_blur() {
    let (w, h) = (1920, 1080);
    let f = stamped(w, h, 0x2468AC, 0x13579B, 128);
    for radius in [1, 2] {
        let b = blur_h(&f, w, h, radius);
        assert_eq!(decode(&b, w, h), Some(code(0x2468AC, 0x13579B)), "radius {radius}");
    }
}

#[test]
fn survives_noise() {
    let (w, h) = (1920, 1080);
    let mut f = stamped(w, h, 0x0F0F0F, 0xF0F0F0, 0);
    let mut rng = Lcg(5);
    for p in f.iter_mut() {
        let n = (rng.byte() % 41) as i32 - 20;
        *p = (*p as i32 + n).clamp(0, 255) as u8;
    }
    assert_eq!(decode(&f, w, h), Some(code(0x0F0F0F, 0xF0F0F0)));
}

#[test]
fn survives_limited_range_video() {
    let (w, h) = (1920, 1080);
    let mut f = stamped(w, h, 0xBEEF01, 0x10CAFE, 100);
    for p in f.iter_mut() {
        *p = (16 + *p as u32 * 219 / 255) as u8;
    }
    assert_eq!(decode(&f, w, h), Some(code(0xBEEF01, 0x10CAFE)));
}

// ---------- must refuse, never guess ----------

#[test]
fn low_contrast_strip_is_rejected() {
    let (w, h) = (1920, 1080);
    let mut f = stamped(w, h, 0x777777, 0x333333, 100);
    for p in f.iter_mut() {
        *p = (100 + *p as u32 * 40 / 255) as u8;
    }
    assert_eq!(decode(&f, w, h), None);
}

#[test]
fn single_flipped_bit_is_rejected_in_every_field() {
    let (w, h) = (1920, 1080);
    // tick cells 2..25, id cells 26..49, crc cells 50..57
    for cell in [2, 10, 25, 26, 40, 49, 50, 57] {
        let mut f = stamped(w, h, 0x3C3C3C, 0xC3C3C3, 0);
        let (x0, x1) = cell_range(w, cell);
        let (_, sh) = strip_size(w).unwrap();
        for y in 0..sh {
            for x in x0..x1 {
                f[y * w + x] = 255 - f[y * w + x];
            }
        }
        assert_eq!(decode(&f, w, h), None, "flipped cell {cell}");
    }
}

#[test]
fn ambiguous_mid_gray_cell_is_rejected() {
    let (w, h) = (1920, 1080);
    for cell in [12, 30] {
        let mut f = stamped(w, h, 0x3C3C3C, 0xC3C3C3, 0);
        let (x0, x1) = cell_range(w, cell);
        let (_, sh) = strip_size(w).unwrap();
        for y in 0..sh {
            for x in x0..x1 {
                f[y * w + x] = 128;
            }
        }
        assert_eq!(decode(&f, w, h), None, "mid-gray cell {cell}");
    }
}

#[test]
fn uniform_frames_are_rejected() {
    let (w, h) = (1920, 1080);
    for v in [0u8, 128, 255] {
        assert_eq!(decode(&vec![v; w * h], w, h), None, "uniform {v}");
    }
}

#[test]
fn random_noise_frames_are_never_decoded() {
    let (w, h) = (640, 360);
    for seed in 0..1000 {
        let f = noisy(w, h, seed);
        assert_eq!(decode(&f, w, h), None, "seed {seed}");
    }
}

#[test]
fn out_of_range_values_are_errors() {
    let (w, h) = (1920, 1080);
    let mut f = vec![0u8; w * h];
    assert_eq!(stamp(&mut f, w, h, 1, MAX_ID + 1), Err(SyncError::IdOutOfRange));
    assert_eq!(stamp(&mut f, w, h, 1, u32::MAX), Err(SyncError::IdOutOfRange));
    assert_eq!(stamp(&mut f, w, h, MAX_TICK + 1, 1), Err(SyncError::TickOutOfRange));
    assert_eq!(stamp(&mut f, w, h, u32::MAX, 1), Err(SyncError::TickOutOfRange));
    // nothing was drawn by the failed calls
    assert!(f.iter().all(|&p| p == 0));
}

#[test]
fn too_small_frames() {
    // cells under 2 px: width < 480
    assert_eq!(strip_size(400), None);
    assert!(strip_size(480).is_some());
    let (w, h) = (400, 225);
    let mut f = vec![0u8; w * h];
    assert_eq!(stamp(&mut f, w, h, 1, 1), Err(SyncError::FrameTooSmall));
    assert_eq!(decode(&f, w, h), None);
    // height shorter than the strip
    let (w, h) = (1920, 4);
    let mut f = vec![0u8; w * h];
    assert_eq!(stamp(&mut f, w, h, 1, 1), Err(SyncError::FrameTooSmall));
    assert_eq!(decode(&f, w, h), None);
}

#[test]
fn wrong_buffer_length() {
    let (w, h) = (1920, 1080);
    let mut short = vec![0u8; w * h - 1];
    assert_eq!(stamp(&mut short, w, h, 1, 1), Err(SyncError::BadBufferLength));
    assert_eq!(decode(&short, w, h), None);
    assert_eq!(decode(&[], 0, 0), None);
}
