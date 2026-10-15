//! Per-frame label store: set / clear / undo, tick-run fill, navigation and an append-only JSONL log.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a frame is labelled as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    /// Not labelled (the default).
    Unset,
    /// Looked at, no fire visible.
    NoFire,
    /// Looked at, deliberately skipped.
    Skip,
    /// A fire sprite: sheet 0 or 1, animation index 0..31, normal or mirrored.
    Fire { sheet: u8, row: u8, mirrored: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelError {
    InvalidSheet(u8),
    InvalidRow(u8),
    FrameOutOfRange { frame: u32, count: u32 },
    /// `fill_range` with `start > end`.
    BadRange { start: u32, end: u32 },
    /// A log line that cannot be parsed (1-based line number).
    Parse { line: usize, msg: String },
}

impl Label {
    /// A validated fire label.
    pub fn fire(sheet: u8, row: u8, mirrored: bool) -> Result<Label, LabelError> {
        let label = Label::Fire { sheet, row, mirrored };
        label.validate()?;
        Ok(label)
    }

    pub fn validate(&self) -> Result<(), LabelError> {
        match *self {
            Label::Fire { sheet, .. } if sheet >= 2 => Err(LabelError::InvalidSheet(sheet)),
            Label::Fire { row, .. } if row >= 32 => Err(LabelError::InvalidRow(row)),
            _ => Ok(()),
        }
    }

    pub fn is_fire(&self) -> bool {
        matches!(self, Label::Fire { .. })
    }
}

/// Result of replaying a log.
#[derive(Debug)]
pub struct Replay {
    pub store: LabelStore,
    /// True when an unparsable final line was dropped (crash while appending).
    pub torn_tail: bool,
}

/// One log line.
#[derive(Serialize, Deserialize)]
struct Entry {
    #[serde(default)]
    version: u32,
    frame: u32,
    label: Label,
}

/// Previous labels of the frames one undo step restores.
type UndoStep = Vec<(u32, Label)>;

#[derive(Debug, Clone)]
pub struct LabelStore {
    frame_count: u32,
    labels: BTreeMap<u32, Label>,
    undo: Vec<UndoStep>,
}

impl LabelStore {
    pub fn new(frame_count: u32) -> LabelStore {
        LabelStore { frame_count, labels: BTreeMap::new(), undo: Vec::new() }
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Label of a frame; `Label::Unset` when none (also for out-of-range frames).
    pub fn get(&self, frame: u32) -> Label {
        self.labels.get(&frame).copied().unwrap_or(Label::Unset)
    }

    fn check_frame(&self, frame: u32) -> Result<(), LabelError> {
        if frame < self.frame_count {
            Ok(())
        } else {
            Err(LabelError::FrameOutOfRange { frame, count: self.frame_count })
        }
    }

    fn put(&mut self, frame: u32, label: Label) {
        if label == Label::Unset {
            self.labels.remove(&frame);
        } else {
            self.labels.insert(frame, label);
        }
    }

    /// Set a label. `Ok(true)` if it changed something (and pushed one undo entry),
    /// `Ok(false)` if it equalled the current label. `Label::Unset` clears.
    pub fn set(&mut self, frame: u32, label: Label) -> Result<bool, LabelError> {
        label.validate()?;
        self.check_frame(frame)?;
        let prev = self.get(frame);
        if prev == label {
            return Ok(false);
        }
        self.undo.push(vec![(frame, prev)]);
        self.put(frame, label);
        Ok(true)
    }

    pub fn clear(&mut self, frame: u32) -> Result<bool, LabelError> {
        self.set(frame, Label::Unset)
    }

    /// Undo the last change (a whole `fill_tick_run` is one change). False when nothing to undo.
    pub fn undo(&mut self) -> bool {
        self.undo_step().is_some()
    }

    /// Undo the last change like `undo`, returning the frames it restored (ascending).
    pub fn undo_step(&mut self) -> Option<Vec<u32>> {
        let step = self.undo.pop()?;
        let mut frames: Vec<u32> = Vec::with_capacity(step.len());
        for (frame, prev) in step {
            self.put(frame, prev);
            frames.push(frame);
        }
        frames.sort_unstable();
        Some(frames)
    }

    /// Label every frame in `start..=end` (inclusive). Returns the number of frames whose label changed.
    /// One undo entry for the whole range, restoring each frame's previous label exactly.
    /// `Label::Unset` clears the range.
    pub fn fill_range(&mut self, start: u32, end: u32, label: Label) -> Result<usize, LabelError> {
        label.validate()?;
        if start > end {
            return Err(LabelError::BadRange { start, end });
        }
        self.check_frame(end)?; // start <= end, so start is in range too
        let mut step: UndoStep = Vec::new();
        for f in start..=end {
            let prev = self.get(f);
            if prev != label {
                step.push((f, prev));
                self.put(f, label);
            }
        }
        let changed = step.len();
        if changed > 0 {
            self.undo.push(step);
        }
        Ok(changed)
    }

    /// First frame strictly after (forward) or before (backward) `from` that is `Unset`.
    /// `NoFire` and `Skip` count as labelled.
    pub fn next_unlabelled(&self, from: u32, forward: bool) -> Option<u32> {
        if forward {
            (from.saturating_add(1)..self.frame_count).find(|f| !self.labels.contains_key(f))
        } else {
            (0..from.min(self.frame_count)).rev().find(|f| !self.labels.contains_key(f))
        }
    }

    /// Number of frames whose label is not `Unset`.
    pub fn labelled_count(&self) -> usize {
        self.labels.len()
    }

    /// All labelled frames in ascending frame order.
    pub fn iter(&self) -> Vec<(u32, Label)> {
        self.labels.iter().map(|(&f, &l)| (f, l)).collect()
    }

    /// Rebuild a store from an append-only log (last line per frame wins).
    /// Blank lines are ignored; an unparsable *last* line is dropped (`torn_tail`);
    /// an unparsable earlier line, an invalid label or a frame >= `frame_count` is an error.
    pub fn replay(text: &str, frame_count: u32) -> Result<Replay, LabelError> {
        let lines: Vec<(usize, &str)> = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim()))
            .filter(|(_, l)| !l.is_empty())
            .collect();
        let mut store = LabelStore::new(frame_count);
        let mut torn_tail = false;
        for (idx, &(line_no, line)) in lines.iter().enumerate() {
            let mut entry: Entry = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(_) if idx + 1 == lines.len() => {
                    torn_tail = true;
                    break;
                }
                Err(e) => return Err(LabelError::Parse { line: line_no, msg: e.to_string() }),
            };
            entry.label.validate()?;
            if entry.version > 2 {
                return Err(LabelError::Parse { line: line_no, msg: "Unsupported label log version".into() });
            }
            // Only the legacy file reader performs v1 -> v2 migration.
            if entry.version < 2 {
                if let Label::Fire { sheet: 0, row, .. } = &mut entry.label {
                    *row = (*row + 16) % 32;
                }
            }
            store.check_frame(entry.frame)?;
            store.put(entry.frame, entry.label);
        }
        Ok(Replay { store, torn_tail })
    }
}

/// One versioned animation-index log entry, without a trailing newline.
pub fn log_line(frame: u32, label: Label) -> String {
    serde_json::to_string(&Entry { version: 2, frame, label }).expect("a plain struct always serialises")
}

/// Clamp a (possibly negative / past-the-end) frame index into `0..count`. `None` when `count == 0`.
pub fn clamp_frame(frame: i64, count: u32) -> Option<u32> {
    if count == 0 {
        None
    } else {
        Some(frame.clamp(0, count as i64 - 1) as u32)
    }
}
