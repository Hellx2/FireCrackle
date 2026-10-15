//! Which frames to decode so the playhead and a window around it are in memory (pure; ffmpeg I/O is M2b).
//!
//! Frame-exact seeking is only possible from a keyframe, so every job starts at the keyframe at or before the
//! first missing frame; the frames between the keyframe and the wanted run are decoded and dropped.

use crate::config::DecodeConfig;
use crate::index::FrameIndex;
use std::collections::HashMap;

/// LRU cache of decoded frames, bounded by bytes. It tracks sizes only; the pixels live elsewhere.
#[derive(Debug, Clone)]
pub struct FrameCache {
    cap_bytes: u64,
    used: u64,
    sizes: HashMap<u32, u64>,
    /// Least recently used first.
    order: Vec<u32>,
}

impl FrameCache {
    pub fn new(cap_bytes: u64) -> FrameCache {
        FrameCache { cap_bytes, used: 0, sizes: HashMap::new(), order: Vec::new() }
    }

    /// Insert or replace a frame (it becomes the most recently used). Returns the frames evicted to stay within
    /// the capacity, oldest first. The frame just inserted is never evicted: a single frame bigger than the whole
    /// cache is kept alone.
    pub fn insert(&mut self, frame: u32, bytes: u64) -> Vec<u32> {
        if let Some(old) = self.sizes.insert(frame, bytes) {
            self.used -= old;
            self.order.retain(|&f| f != frame);
        }
        self.used += bytes;
        self.order.push(frame);
        let mut evicted = Vec::new();
        while self.used > self.cap_bytes && self.order.len() > 1 {
            let oldest = self.order.remove(0);
            self.used -= self.sizes.remove(&oldest).unwrap_or(0);
            evicted.push(oldest);
        }
        evicted
    }

    pub fn contains(&self, frame: u32) -> bool {
        self.sizes.contains_key(&frame)
    }

    /// Mark a cached frame as just used. False when it is not cached.
    pub fn touch(&mut self, frame: u32) -> bool {
        if !self.sizes.contains_key(&frame) {
            return false;
        }
        self.order.retain(|&f| f != frame);
        self.order.push(frame);
        true
    }

    pub fn remove(&mut self, frame: u32) -> bool {
        match self.sizes.remove(&frame) {
            Some(bytes) => {
                self.used -= bytes;
                self.order.retain(|&f| f != frame);
                true
            }
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        self.sizes.len()
    }

    pub fn used_bytes(&self) -> u64 {
        self.used
    }

    pub fn capacity_bytes(&self) -> u64 {
        self.cap_bytes
    }

    /// Cached frames in ascending order.
    pub fn frames(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.sizes.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Change the capacity; frames are evicted (oldest first, returned) until the cache fits. One frame is
    /// always kept.
    pub fn set_capacity(&mut self, cap_bytes: u64) -> Vec<u32> {
        self.cap_bytes = cap_bytes;
        let mut evicted = Vec::new();
        while self.used > self.cap_bytes && self.order.len() > 1 {
            let oldest = self.order.remove(0);
            self.used -= self.sizes.remove(&oldest).unwrap_or(0);
            evicted.push(oldest);
        }
        evicted
    }

    pub fn clear(&mut self) {
        self.sizes.clear();
        self.order.clear();
        self.used = 0;
    }
}

/// One ffmpeg run: seek to `keyframe`, decode `decode_count` frames, keep `first..=last`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodeJob {
    pub keyframe: u32,
    pub seek_pts: f64,
    pub first: u32,
    pub last: u32,
    /// Frames to decode from the keyframe through `last` (the ones before `first` are dropped).
    pub decode_count: u32,
    /// The keyframe is further than `max_gop_frames` before `first`: show a spinner.
    pub slow: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// The wanted frame is already cached.
    pub want_cached: bool,
    pub job: Option<DecodeJob>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    EmptyIndex,
}

/// Plan the decode for `want` (clamped to the last frame).
///
/// - `want` missing: decode the missing run around it, within the window `[want - back, want + ahead]`.
/// - `want` cached: prefetch only when the cached frames ahead (or behind) are under `prefetch_fraction` of the
///   window on that side, and then fetch the whole missing run, so playback does not start ffmpeg per frame.
pub fn plan(index: &FrameIndex, cache: &FrameCache, want: u32, cfg: &DecodeConfig) -> Result<Plan, PlanError> {
    let n = index.frames.len();
    if n == 0 {
        return Err(PlanError::EmptyIndex);
    }
    let last = (n - 1) as u32;
    let want = want.min(last);
    let lo = want.saturating_sub(cfg.window_back);
    let hi = want.saturating_add(cfg.window_ahead).min(last);

    // a job for the missing run `a..=b`, started at the keyframe governing `a`
    let job = |a: u32, b: u32| {
        let keyframe = index.keyframe_at_or_before(a).unwrap_or(0);
        DecodeJob {
            keyframe,
            seek_pts: index.frames[keyframe as usize].pts,
            first: a,
            last: b,
            decode_count: b - keyframe + 1,
            slow: a - keyframe > cfg.max_gop_frames,
        }
    };

    if !cache.contains(want) {
        let mut a = want;
        while a > lo && !cache.contains(a - 1) {
            a -= 1;
        }
        let mut b = want;
        while b < hi && !cache.contains(b + 1) {
            b += 1;
        }
        return Ok(Plan { want_cached: false, job: Some(job(a, b)) });
    }

    // the wanted frame is cached: top the window up only when a side has run low
    let ahead_target = hi - want;
    let mut ahead_cached = 0;
    while ahead_cached < ahead_target && cache.contains(want + ahead_cached + 1) {
        ahead_cached += 1;
    }
    if ahead_target > 0 && (ahead_cached as f64) < cfg.prefetch_fraction * ahead_target as f64 {
        let a = want + ahead_cached + 1;
        let mut b = a;
        while b < hi && !cache.contains(b + 1) {
            b += 1;
        }
        return Ok(Plan { want_cached: true, job: Some(job(a, b)) });
    }

    let behind_target = want - lo;
    let mut behind_cached = 0;
    while behind_cached < behind_target && cache.contains(want - behind_cached - 1) {
        behind_cached += 1;
    }
    if behind_target > 0 && (behind_cached as f64) < cfg.prefetch_fraction * behind_target as f64 {
        let b = want - behind_cached - 1;
        let mut a = b;
        while a > lo && !cache.contains(a - 1) {
            a -= 1;
        }
        return Ok(Plan { want_cached: true, job: Some(job(a, b)) });
    }
    Ok(Plan { want_cached: true, job: None })
}
