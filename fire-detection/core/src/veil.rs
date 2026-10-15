//! Detecting the vanilla GUI background veil. See tests/veil.rs for the numbers.

/// Alpha at the top and bottom row of the veil: 192/255 and 208/255 (fillGradient 0xC0101010 -> 0xD0101010).
pub const A_TOP: f64 = 192.0 / 255.0;
pub const A_BOTTOM: f64 = 208.0 / 255.0;
/// Colour of the veil (16, 16, 16).
pub const COLOUR: f64 = 16.0;
/// Pixels outside the legal veiled range tolerated (widgets, text, compression): the frame is veiled when at least this
/// share of the pixels is inside it.
const MIN_SHARE: f64 = 0.90;
const SLACK: f64 = 2.0;

/// True when the frame (rows `y0..h`) looks like the game's GUI background veil was drawn over it: out = 16 * a + p * (1 - a)
/// bounds every channel to [16 * a_top, 16 * a + 255 * (1 - a)] (about 12..76 at the top, 13..60 at the bottom), so a veiled
/// frame has (almost) no pixel outside that band, whereas a real dark scene has true blacks and a real bright one has values
/// far above it.
pub fn detect_veil(rgb: &[u8], w: usize, h: usize, y0: usize) -> Result<bool, String> {
    if w == 0 || h == 0 || rgb.len() != w * h * 3 {
        return Err(format!("frame must be {w}x{h} RGB24 ({} bytes), got {}", w * h * 3, rgb.len()));
    }
    if y0 >= h {
        return Err(format!("no rows left: y0 {y0} for height {h}"));
    }
    let lo = COLOUR * A_TOP - SLACK;
    let mut inside = 0usize;
    let rows = h - y0;
    for y in y0..h {
        let a = A_TOP + (A_BOTTOM - A_TOP) * (y as f64 + 0.5) / h as f64;
        let hi = COLOUR * a + 255.0 * (1.0 - a) + SLACK;
        for px in rgb[y * w * 3..(y + 1) * w * 3].chunks_exact(3) {
            if px.iter().all(|&c| (c as f64) >= lo && (c as f64) <= hi) {
                inside += 1;
            }
        }
    }
    Ok(inside as f64 >= MIN_SHARE * (rows * w) as f64)
}
