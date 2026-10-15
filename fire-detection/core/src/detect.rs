//! Classical baseline: template matching of fire parts given their (labelled) box. See DETECT_PLAN.md.

pub const ROWS: usize = 32;
const TEX: usize = 16;
const SHEET_LEN: usize = TEX * TEX * ROWS * 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Candidate {
    pub sheet: u8,
    pub mirror: bool,
    pub row: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Scored {
    pub cand: Candidate,
    pub score: f64,
}

struct Template {
    cand: Candidate,
    rgba: Vec<u8>, // 16x16, already mirrored if needed
    bw: usize,
    bh: usize,
    /// opaque pixels inside the bbox: (x, y, luma) with x, y relative to the bbox
    px: Vec<(usize, usize, f64)>,
    mean: f64,
    var: f64,
}

pub struct Templates {
    items: Vec<Template>,
}

fn luma(r: f64, g: f64, b: f64) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}

impl Templates {
    /// Both sheets as raw RGBA, 16 wide and 32 frames of 16 rows each. Order: sheet 0 normal rows 0..32, sheet 0
    /// mirrored, sheet 1 normal, sheet 1 mirrored.
    pub fn from_rgba(s0: &[u8], s1: &[u8]) -> Result<Templates, String> {
        for (n, s) in [(0, s0), (1, s1)] {
            if s.len() != SHEET_LEN {
                return Err(format!(
                    "fire_{n} must be {SHEET_LEN} bytes of RGBA, got {}",
                    s.len()
                ));
            }
        }
        let mut items = Vec::with_capacity(4 * ROWS);
        for (sheet, data) in [s0, s1].iter().enumerate() {
            for mirror in [false, true] {
                for row in 0..ROWS {
                    let mut rgba = vec![0u8; TEX * TEX * 4];
                    for y in 0..TEX {
                        for x in 0..TEX {
                            let sx = if mirror { TEX - 1 - x } else { x };
                            let i = ((row * TEX + y) * TEX + sx) * 4;
                            rgba[(y * TEX + x) * 4..(y * TEX + x) * 4 + 4]
                                .copy_from_slice(&data[i..i + 4]);
                        }
                    }
                    items.push(Template::new(
                        Candidate {
                            sheet: sheet as u8,
                            mirror,
                            row: row as u8,
                        },
                        rgba,
                    )?);
                }
            }
        }
        Ok(Templates { items })
    }

    pub fn candidates(&self) -> Vec<Candidate> {
        self.items.iter().map(|t| t.cand).collect()
    }

    /// True when both candidates show exactly the same opaque pixels (no matcher can tell them apart).
    pub fn same_pixels(&self, a: Candidate, b: Candidate) -> bool {
        let (Some(a), Some(b)) = (self.find(a), self.find(b)) else {
            return false;
        };
        (0..TEX * TEX).all(|p| {
            let (x, y) = (&a.rgba[p * 4..p * 4 + 4], &b.rgba[p * 4..p * 4 + 4]);
            (x[3] == 0 && y[3] == 0) || (x[3] != 0 && y[3] != 0 && x[..3] == y[..3])
        })
    }

    fn find(&self, c: Candidate) -> Option<&Template> {
        self.items.iter().find(|t| t.cand == c)
    }
}

impl Template {
    fn new(cand: Candidate, rgba: Vec<u8>) -> Result<Template, String> {
        let (mut x0, mut y0, mut x1, mut y1) = (TEX, TEX, 0, 0);
        for y in 0..TEX {
            for x in 0..TEX {
                if rgba[(y * TEX + x) * 4 + 3] != 0 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
        }
        if x1 <= x0 {
            return Err(format!("template {cand:?} has no opaque pixel"));
        }
        let mut px = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * TEX + x) * 4;
                if rgba[i + 3] != 0 {
                    px.push((
                        x - x0,
                        y - y0,
                        luma(rgba[i] as f64, rgba[i + 1] as f64, rgba[i + 2] as f64),
                    ));
                }
            }
        }
        let n = px.len() as f64;
        let mean = px.iter().map(|p| p.2).sum::<f64>() / n;
        let var = px.iter().map(|p| (p.2 - mean) * (p.2 - mean)).sum::<f64>();
        Ok(Template {
            cand,
            rgba,
            bw: x1 - x0,
            bh: y1 - y0,
            px,
            mean,
            var,
        })
    }
}

/// Ranks all candidates for the part whose box (frame pixels, `[x0, y0, x1, y1]`) is given, best first. The box is
/// clamped to the frame. Score is the zero-mean normalised correlation (-1..1) of luma over the candidate's opaque
/// pixels, the crop being resampled to the candidate's opaque bounding box. Ties keep candidate order.
pub fn classify(
    t: &Templates,
    frame: &[u8],
    w: usize,
    h: usize,
    bbox: [f64; 4],
) -> Result<Vec<Scored>, String> {
    if w == 0 || h == 0 || frame.len() != w * h * 3 {
        return Err(format!(
            "frame must be {w}x{h} RGB24 ({} bytes), got {}",
            w * h * 3,
            frame.len()
        ));
    }
    if bbox.iter().any(|v| !v.is_finite()) {
        return Err("box has a non-finite coordinate".into());
    }
    if bbox[2] <= bbox[0] || bbox[3] <= bbox[1] {
        return Err(format!("box has no area: {bbox:?}"));
    }
    let x0 = bbox[0].max(0.0);
    let y0 = bbox[1].max(0.0);
    let x1 = bbox[2].min(w as f64);
    let y1 = bbox[3].min(h as f64);
    if x1 <= x0 || y1 <= y0 {
        return Err(format!("box is outside the {w}x{h} frame: {bbox:?}"));
    }
    let (bw, bh) = (x1 - x0, y1 - y0);

    // integral image of luma over the crop's pixel rectangle
    let ix0 = x0.floor() as usize;
    let iy0 = y0.floor() as usize;
    let ix1 = (x1.ceil() as usize).min(w);
    let iy1 = (y1.ceil() as usize).min(h);
    let (cw, ch) = (ix1 - ix0, iy1 - iy0);
    let mut integral = vec![0.0f64; (cw + 1) * (ch + 1)];
    for y in 0..ch {
        let mut run = 0.0;
        for x in 0..cw {
            let o = ((iy0 + y) * w + ix0 + x) * 3;
            run += luma(frame[o] as f64, frame[o + 1] as f64, frame[o + 2] as f64);
            integral[(y + 1) * (cw + 1) + x + 1] = integral[y * (cw + 1) + x + 1] + run;
        }
    }
    let cell_mean = |a: f64, b: f64, c: f64, d: f64| -> f64 {
        // pixel columns/rows [a, c) x [b, d) in frame coordinates, at least one pixel
        let xa = (a.floor() as usize).clamp(ix0, ix1 - 1);
        let ya = (b.floor() as usize).clamp(iy0, iy1 - 1);
        let xb = (c.ceil() as usize).clamp(xa + 1, ix1);
        let yb = (d.ceil() as usize).clamp(ya + 1, iy1);
        let (xa, xb, ya, yb) = (xa - ix0, xb - ix0, ya - iy0, yb - iy0);
        let s = integral[yb * (cw + 1) + xb]
            - integral[ya * (cw + 1) + xb]
            - integral[yb * (cw + 1) + xa]
            + integral[ya * (cw + 1) + xa];
        s / ((xb - xa) * (yb - ya)) as f64
    };

    let mut out: Vec<Scored> = Vec::with_capacity(t.items.len());
    let mut sample = Vec::new();
    for tpl in &t.items {
        sample.clear();
        for &(px, py, _) in &tpl.px {
            let a = x0 + px as f64 * bw / tpl.bw as f64;
            let c = x0 + (px + 1) as f64 * bw / tpl.bw as f64;
            let b = y0 + py as f64 * bh / tpl.bh as f64;
            let d = y0 + (py + 1) as f64 * bh / tpl.bh as f64;
            sample.push(cell_mean(a, b, c, d));
        }
        let n = sample.len() as f64;
        let mean = sample.iter().sum::<f64>() / n;
        let var = sample.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>();
        let score = if var < 1e-9 || tpl.var < 1e-9 {
            -1.0
        } else {
            let cov: f64 = sample
                .iter()
                .zip(&tpl.px)
                .map(|(s, p)| (s - mean) * (p.2 - tpl.mean))
                .sum();
            (cov / (var * tpl.var).sqrt()).clamp(-1.0, 1.0)
        };
        out.push(Scored {
            cand: tpl.cand,
            score,
        });
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(out)
}
