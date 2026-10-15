//! Temporal flicker features. See DETECT_PLAN.md ("round 6").
use crate::presence::Palette;

fn luma(p: &[u8]) -> f64 {
    0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64
}

fn check_pair(a: &[u8], b: &[u8], w: usize, h: usize, y0: usize) -> Result<(), String> {
    if w == 0 || h == 0 || a.len() != w * h * 3 || b.len() != w * h * 3 {
        return Err(format!(
            "both frames must be {w}x{h} RGB24 ({} bytes), got {} and {}",
            w * h * 3,
            a.len(),
            b.len()
        ));
    }
    if y0 > h {
        return Err(format!("y0 {y0} is past the frame height {h}"));
    }
    Ok(())
}

/// Pixels (rows `y0..h`, row-major) that are palette-coloured in BOTH frames and whose luma differs by at least `min_dl`.
pub fn flicker_mask(
    a: &[u8],
    b: &[u8],
    w: usize,
    h: usize,
    y0: usize,
    pal: &Palette,
    min_dl: f64,
) -> Result<Vec<bool>, String> {
    check_pair(a, b, w, h, y0)?;
    if !min_dl.is_finite() {
        return Err(format!("min_dl must be finite: {min_dl}"));
    }
    let ma = pal.mask(a, w, h, y0, h)?;
    let mb = pal.mask(b, w, h, y0, h)?;
    let off = y0 * w * 3;
    Ok((0..ma.len())
        .map(|i| {
            ma[i] && mb[i] && (luma(&a[off + i * 3..off + i * 3 + 3]) - luma(&b[off + i * 3..off + i * 3 + 3])).abs() >= min_dl
        })
        .collect())
}

/// Mean |luma change| over pixels (rows `y0..h`, every `step`-th row and column) that are palette-coloured in neither
/// frame: how much the camera / scene moved. 0 when there is no such pixel.
pub fn bg_motion(a: &[u8], b: &[u8], w: usize, h: usize, y0: usize, pal: &Palette, step: usize) -> Result<f64, String> {
    check_pair(a, b, w, h, y0)?;
    if step == 0 {
        return Err("step must be >= 1".into());
    }
    let ma = pal.mask(a, w, h, y0, h)?;
    let mb = pal.mask(b, w, h, y0, h)?;
    let off = y0 * w * 3;
    let (mut sum, mut n) = (0.0f64, 0usize);
    for y in (0..h - y0).step_by(step) {
        for x in (0..w).step_by(step) {
            let i = y * w + x;
            if !ma[i] && !mb[i] {
                sum += (luma(&a[off + i * 3..off + i * 3 + 3]) - luma(&b[off + i * 3..off + i * 3 + 3])).abs();
                n += 1;
            }
        }
    }
    Ok(if n == 0 { 0.0 } else { sum / n as f64 })
}

/// `history`: (frame, tick) pairs, oldest first. Index of the latest entry whose tick is `tick - 1`, if any.
pub fn find_partner(history: &[(u64, u64)], tick: u64) -> Option<usize> {
    let prev = tick.checked_sub(1)?;
    history.iter().rposition(|&(_, t)| t == prev)
}
