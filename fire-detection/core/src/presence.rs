//! Stage 1 of the baseline: is there fire in the frame? See DETECT_PLAN.md ("Stage 1").
//!
//! The fire textures use a small, smooth palette (orange -> yellow -> white). A 256^3 bit table marks every RGB within
//! `dist` (Euclidean) of a palette colour; a frame's score is the number of its pixels that hit the table.

pub struct Palette {
    table: Vec<u64>, // 2^24 bits
}

fn bit(r: usize, g: usize, b: usize) -> usize {
    (r << 16) | (g << 8) | b
}

impl Palette {
    /// `sheets`: raw RGBA images (any size, length a multiple of 4); every pixel with alpha > 0 contributes its colour.
    pub fn from_rgba(sheets: &[&[u8]], dist: f64) -> Result<Palette, String> {
        if !dist.is_finite() || dist < 0.0 {
            return Err(format!("distance must be finite and >= 0: {dist}"));
        }
        let mut colours: Vec<[u8; 3]> = Vec::new();
        for s in sheets {
            if s.len() % 4 != 0 {
                return Err(format!(
                    "RGBA data must be a multiple of 4 bytes, got {}",
                    s.len()
                ));
            }
            for p in s.as_chunks::<4>().0 {
                if p[3] != 0 && !colours.contains(&[p[0], p[1], p[2]]) {
                    colours.push([p[0], p[1], p[2]]);
                }
            }
        }
        if colours.is_empty() {
            return Err("no opaque pixel to build a palette from".into());
        }
        let d2 = (dist * dist).floor() as i64;
        let reach = dist.floor() as i64;
        let mut table = vec![0u64; (1 << 24) / 64];
        for c in &colours {
            let (cr, cg, cb) = (c[0] as i64, c[1] as i64, c[2] as i64);
            for r in (cr - reach).max(0)..=(cr + reach).min(255) {
                for g in (cg - reach).max(0)..=(cg + reach).min(255) {
                    let rg = (r - cr) * (r - cr) + (g - cg) * (g - cg);
                    if rg > d2 {
                        continue;
                    }
                    for b in (cb - reach).max(0)..=(cb + reach).min(255) {
                        if rg + (b - cb) * (b - cb) <= d2 {
                            let i = bit(r as usize, g as usize, b as usize);
                            table[i >> 6] |= 1 << (i & 63);
                        }
                    }
                }
            }
        }
        Ok(Palette { table })
    }

    /// Per-pixel hit mask for rows `y0..y1` (row-major, `w * (y1 - y0)` entries).
    pub fn mask(
        &self,
        rgb: &[u8],
        w: usize,
        h: usize,
        y0: usize,
        y1: usize,
    ) -> Result<Vec<bool>, String> {
        if w == 0 || h == 0 || rgb.len() != w * h * 3 {
            return Err(format!(
                "frame must be {w}x{h} RGB24 ({} bytes), got {}",
                w * h * 3,
                rgb.len()
            ));
        }
        if y0 > y1 || y1 > h {
            return Err(format!("bad row range {y0}..{y1} for height {h}"));
        }
        Ok(rgb[y0 * w * 3..y1 * w * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| {
                let i = bit(p[0] as usize, p[1] as usize, p[2] as usize);
                (self.table[i >> 6] >> (i & 63)) & 1 == 1
            })
            .collect())
    }

    /// Number of pixels of an RGB24 frame whose colour is within the palette distance.
    pub fn count(&self, rgb: &[u8], w: usize, h: usize) -> Result<usize, String> {
        self.count_rows(rgb, w, h, 0, h)
    }

    /// Same, for rows `y0..y1` only (to leave the sync strip out).
    pub fn count_rows(
        &self,
        rgb: &[u8],
        w: usize,
        h: usize,
        y0: usize,
        y1: usize,
    ) -> Result<usize, String> {
        if w == 0 || h == 0 || rgb.len() != w * h * 3 {
            return Err(format!(
                "frame must be {w}x{h} RGB24 ({} bytes), got {}",
                w * h * 3,
                rgb.len()
            ));
        }
        if y0 > y1 || y1 > h {
            return Err(format!("bad row range {y0}..{y1} for height {h}"));
        }
        let mut n = 0usize;
        for p in rgb[y0 * w * 3..y1 * w * 3].as_chunks::<3>().0 {
            let i = bit(p[0] as usize, p[1] as usize, p[2] as usize);
            n += ((self.table[i >> 6] >> (i & 63)) & 1) as usize;
        }
        Ok(n)
    }
}
