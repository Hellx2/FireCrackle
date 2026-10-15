//! Patch-level detector pieces. See DETECT_PLAN.md ("round 3").
use crate::features::blob_stats;
use crate::json::{parse, Value};
use crate::presence::Palette;

pub const PATCH: usize = 16;
pub const NFEAT: usize = 12;
pub const FEATURE_NAMES: [&str; NFEAT] =
    ["pal", "strict", "bright", "white", "dark", "lum", "lumstd", "grad", "big", "sat", "warm", "ring"];

const BIG_STEP: f32 = 30.0;

pub struct PatchGrid {
    pub gx: usize,
    pub gy: usize,
    /// Row-major, `gx * gy` entries; features in `FEATURE_NAMES` order.
    pub feats: Vec<[f32; NFEAT]>,
}

/// Cuts rows `y0..h` of an RGB24 frame into 16x16 patches (partial patches at the right and bottom are dropped) and
/// computes the features of each.
pub fn frame_patch_features(
    rgb: &[u8],
    w: usize,
    h: usize,
    y0: usize,
    loose: &Palette,
    strict: &Palette,
) -> Result<PatchGrid, String> {
    if w == 0 || h == 0 || rgb.len() != w * h * 3 {
        return Err(format!("frame must be {w}x{h} RGB24 ({} bytes), got {}", w * h * 3, rgb.len()));
    }
    if y0 > h {
        return Err(format!("y0 {y0} is past the frame height {h}"));
    }
    let (gx, gy) = (w / PATCH, (h - y0) / PATCH);
    if gx == 0 || gy == 0 {
        return Err(format!("no whole {PATCH}x{PATCH} patch fits in {w}x{} px", h - y0));
    }
    let rows = h - y0;
    let loose_mask = loose.mask(rgb, w, h, y0, h)?;
    let strict_mask = strict.mask(rgb, w, h, y0, h)?;
    let px = &rgb[y0 * w * 3..];
    let luma: Vec<f32> = px
        .chunks_exact(3)
        .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
        .collect();
    debug_assert_eq!(luma.len(), w * rows);

    let n = (PATCH * PATCH) as f32;
    let mut feats = vec![[0.0f32; NFEAT]; gx * gy];
    for py in 0..gy {
        for pxi in 0..gx {
            let (mut pal, mut st, mut bright, mut white, mut dark) = (0u32, 0u32, 0u32, 0u32, 0u32);
            let (mut sum, mut sum2, mut sat, mut warm) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            let (mut dx, mut ndx, mut dy, mut ndy, mut big) = (0.0f32, 0u32, 0.0f32, 0u32, 0u32);
            for y in 0..PATCH {
                for x in 0..PATCH {
                    let gxp = pxi * PATCH + x;
                    let gyp = py * PATCH + y;
                    let i = gyp * w + gxp;
                    let l = luma[i];
                    pal += loose_mask[i] as u32;
                    st += strict_mask[i] as u32;
                    if loose_mask[i] && l >= 170.0 {
                        bright += 1;
                    }
                    white += (l >= 235.0) as u32;
                    dark += (l < 40.0) as u32;
                    sum += l;
                    sum2 += l * l;
                    let o = i * 3;
                    let (r, g, b) = (px[o] as f32, px[o + 1] as f32, px[o + 2] as f32);
                    let mx = r.max(g).max(b);
                    let mn = r.min(g).min(b);
                    if mx > 0.0 {
                        sat += (mx - mn) / mx;
                    }
                    warm += (r - b) / 255.0;
                    if x + 1 < PATCH {
                        let d = (l - luma[i + 1]).abs();
                        dx += d;
                        ndx += 1;
                        big += (d > BIG_STEP) as u32;
                    }
                    if y + 1 < PATCH {
                        dy += (l - luma[i + w]).abs();
                        ndy += 1;
                    }
                }
            }
            let mean = sum / n;
            let var = (sum2 / n - mean * mean).max(0.0);
            feats[py * gx + pxi] = [
                pal as f32 / n,
                st as f32 / n,
                bright as f32 / n,
                white as f32 / n,
                dark as f32 / n,
                mean / 255.0,
                var.sqrt() / 255.0,
                (dx / ndx as f32 + dy / ndy as f32) / 255.0,
                big as f32 / ndx as f32,
                sat / n,
                warm / n,
                0.0,
            ];
        }
    }
    // ring: mean `pal` of the existing 8 neighbours
    let pal_of: Vec<f32> = feats.iter().map(|f| f[0]).collect();
    for py in 0..gy {
        for pxi in 0..gx {
            let (mut s, mut c) = (0.0f32, 0u32);
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (pxi as i32 + dx, py as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < gx && (ny as usize) < gy {
                        s += pal_of[ny as usize * gx + nx as usize];
                        c += 1;
                    }
                }
            }
            feats[py * gx + pxi][11] = if c > 0 { s / c as f32 } else { 0.0 };
        }
    }
    Ok(PatchGrid { gx, gy, feats })
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PatchTruth {
    Pos,
    Neg,
    Skip,
}

const POS_VIS: f64 = 0.3;
const NEG_MARGIN: f64 = 8.0;

/// Truth of the patch whose top-left pixel is `(px, py)`, from label boxes `([x0, y0, x1, y1], vis)` in the same pixel
/// space. Positive: a box with vis >= 0.3 covers at least half the patch. Negative: no box (any vis) within 8 px of it.
/// Anything else is skipped.
pub fn patch_truth(boxes: &[([f64; 4], f64)], px: usize, py: usize) -> PatchTruth {
    let (x0, y0) = (px as f64, py as f64);
    let (x1, y1) = (x0 + PATCH as f64, y0 + PATCH as f64);
    let mut near = false;
    for (b, vis) in boxes {
        let gap_x = (b[0] - x1).max(x0 - b[2]).max(0.0);
        let gap_y = (b[1] - y1).max(y0 - b[3]).max(0.0);
        // NaN coordinates make every comparison false: such a box is ignored
        if gap_x < NEG_MARGIN && gap_y < NEG_MARGIN {
            near = true;
        }
        if *vis >= POS_VIS {
            let ox = (x1.min(b[2]) - x0.max(b[0])).max(0.0);
            let oy = (y1.min(b[3]) - y0.max(b[1])).max(0.0);
            if ox * oy >= (PATCH * PATCH) as f64 / 2.0 {
                return PatchTruth::Pos;
            }
        }
    }
    if near {
        PatchTruth::Skip
    } else {
        PatchTruth::Neg
    }
}

enum Node {
    Leaf(f32),
    Split { f: usize, t: f32, l: usize, r: usize },
}

/// A decision tree exported from scikit-learn (preorder: children come after their parent).
pub struct PatchModel {
    nodes: Vec<Node>,
}

impl PatchModel {
    /// `{"nodes":[{"f":0,"t":0.5,"l":1,"r":2},{"leaf":0.1},...]}`; `x[f] <= t` goes to `l`.
    pub fn from_json(text: &str) -> Result<PatchModel, String> {
        let v = parse(text)?;
        let arr = v.get("nodes").and_then(Value::arr).ok_or("model needs a \"nodes\" array")?;
        if arr.is_empty() {
            return Err("model has no nodes".into());
        }
        let mut nodes = Vec::with_capacity(arr.len());
        for (i, n) in arr.iter().enumerate() {
            if let Some(leaf) = n.get("leaf") {
                nodes.push(Node::Leaf(leaf.num().ok_or(format!("node {i}: leaf must be a number"))? as f32));
                continue;
            }
            let num = |k: &str| n.get(k).and_then(Value::num).ok_or(format!("node {i}: missing \"{k}\""));
            let (f, t, l, r) = (num("f")?, num("t")?, num("l")?, num("r")?);
            let idx = |x: f64, what: &str| -> Result<usize, String> {
                if x < 0.0 || x.fract() != 0.0 {
                    return Err(format!("node {i}: bad {what} {x}"));
                }
                Ok(x as usize)
            };
            let (f, l, r) = (idx(f, "feature")?, idx(l, "left child")?, idx(r, "right child")?);
            if f >= NFEAT {
                return Err(format!("node {i}: feature {f} out of range"));
            }
            if l <= i || r <= i || l >= arr.len() || r >= arr.len() {
                return Err(format!("node {i}: children must come after the node and exist"));
            }
            nodes.push(Node::Split { f, t: t as f32, l, r });
        }
        Ok(PatchModel { nodes })
    }

    pub fn predict(&self, x: &[f32; NFEAT]) -> f32 {
        let mut i = 0;
        loop {
            match &self.nodes[i] {
                Node::Leaf(v) => return *v,
                Node::Split { f, t, l, r } => i = if x[*f] <= *t { *l } else { *r },
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Pooled {
    pub positives: usize,
    pub largest_cluster: usize,
}

/// Frame-level summary of a grid of positive/negative patch decisions.
pub fn pool_patches(positive: &[bool], gx: usize, gy: usize) -> Result<Pooled, String> {
    let b = blob_stats(positive, gx, gy, 1)?;
    Ok(Pooled { positives: b.total, largest_cluster: b.largest })
}
