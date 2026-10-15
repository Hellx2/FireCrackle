//! Reducing a long phase series to what a chart can draw, without hiding anything suspicious.

use crate::phase::{Point, STEPS};

const CIRCLE: f64 = STEPS as f64;

/// Signed shortest offset from `reference` to `phase` on the circle, in [-16, 16).
fn offset(phase: f64, reference: f64) -> f64 {
    (phase - reference + CIRCLE / 2.0).rem_euclid(CIRCLE) - CIRCLE / 2.0
}

/// Reduce `points` (in frame order) to at most `max_points`, keeping the very first and last point.
///
/// The points between them are split into equal buckets (by position); each bucket keeps its first point and the two
/// whose phase is furthest above and below the first one's, measured round the circle. A steady stretch therefore
/// draws as a band as wide as its real scatter, and a lone outlier or a stall is never averaged away. Input that
/// already fits is returned unchanged. Caps too small for any bucket (< 5) give just the ends (a cap of 1: the first
/// point; 0: nothing).
pub fn decimate(points: &[Point], max_points: usize) -> Vec<Point> {
    let n = points.len();
    if max_points == 0 || n == 0 {
        return Vec::new();
    }
    if n <= max_points {
        return points.to_vec();
    }
    if max_points == 1 {
        return vec![points[0]];
    }
    let interior = n - 2;
    let buckets = ((max_points - 2) / 3).min(interior);
    let mut keep = vec![0usize];
    for b in 0..buckets {
        let lo = 1 + b * interior / buckets;
        let hi = 1 + (b + 1) * interior / buckets; // exclusive
        keep.push(lo);
        let reference = points[lo].phase;
        let (mut min_i, mut max_i) = (lo, lo);
        let (mut min_v, mut max_v) = (0.0f64, 0.0f64);
        for i in lo..hi {
            let v = offset(points[i].phase, reference);
            if v < min_v {
                min_v = v;
                min_i = i;
            }
            if v > max_v {
                max_v = v;
                max_i = i;
            }
        }
        keep.push(min_i);
        keep.push(max_i);
    }
    keep.push(n - 1);
    keep.sort_unstable();
    keep.dedup();
    keep.into_iter().map(|i| points[i]).collect()
}
