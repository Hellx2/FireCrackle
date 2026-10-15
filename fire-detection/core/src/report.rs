//! Output rows and the run summary of `fire-decode`.

use crate::sync::SyncCode;

/// What the label log said about a decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelResult {
    /// No label file was given.
    NotRequested,
    /// A label file was given but has no row for this frame (or the frame could not be decoded).
    Missing,
    /// The row for the decoded id: its tick (full, from the log) and its raw text.
    Found { tick: u64, line: String },
}

const TICK_MASK: u64 = 0xFF_FFFF;

fn tick_ok(code: SyncCode, tick: u64) -> bool {
    tick & TICK_MASK == code.tick as u64
}

fn field<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("null".to_string(), |x| x.to_string())
}

/// One JSONL output line for video frame `n`.
pub fn format_row(n: u64, code: Option<SyncCode>, label: &LabelResult) -> String {
    let tick = field(code.map(|c| c.tick));
    let id = field(code.map(|c| c.frame_id));
    match label {
        LabelResult::NotRequested => format!("{{\"n\":{n},\"tick\":{tick},\"f\":{id}}}"),
        LabelResult::Missing => format!("{{\"n\":{n},\"tick\":{tick},\"f\":{id},\"tick_ok\":null,\"label\":null}}"),
        LabelResult::Found { tick: label_tick, line } => {
            let ok = field(code.map(|c| tick_ok(c, *label_tick)));
            format!("{{\"n\":{n},\"tick\":{tick},\"f\":{id},\"tick_ok\":{ok},\"label\":{line}}}")
        }
    }
}

#[derive(Debug, Default)]
pub struct Summary {
    pub frames: u64,
    pub decoded: u64,
    pub undecodable: u64,
    pub matched: u64,
    pub unmatched: u64,
    pub tick_mismatch: u64,
    /// A decoded id equal to the previous decoded one: a duplicated frame.
    pub id_repeats: u64,
    /// A decoded id lower than the previous decoded one: dropped, reordered or spliced footage.
    pub id_decreases: u64,
    last_id: Option<u32>,
}

impl Summary {
    pub fn record(&mut self, code: Option<SyncCode>, label: &LabelResult) {
        self.frames += 1;
        let Some(c) = code else {
            self.undecodable += 1;
            return;
        };
        self.decoded += 1;
        if let Some(prev) = self.last_id {
            if c.frame_id == prev {
                self.id_repeats += 1;
            } else if c.frame_id < prev {
                self.id_decreases += 1;
            }
        }
        self.last_id = Some(c.frame_id);
        match label {
            LabelResult::NotRequested => {}
            LabelResult::Missing => self.unmatched += 1,
            LabelResult::Found { tick, .. } => {
                self.matched += 1;
                if !tick_ok(c, *tick) {
                    self.tick_mismatch += 1;
                }
            }
        }
    }

    pub fn to_line(&self) -> String {
        format!(
            "frames={} decoded={} undecodable={} matched={} unmatched={} tick_mismatch={} id_repeats={} id_decreases={}",
            self.frames, self.decoded, self.undecodable, self.matched, self.unmatched, self.tick_mismatch,
            self.id_repeats, self.id_decreases
        )
    }
}
