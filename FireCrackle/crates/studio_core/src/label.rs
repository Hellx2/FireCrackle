//! Per-frame label store: set / clear / undo, tick-run fill, navigation and an append-only JSONL log.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    FrameOutOfRange {
        frame: u32,
        count: u32,
    },
    /// `fill_range` with `start > end`.
    BadRange {
        start: u32,
        end: u32,
    },
    /// A log line that cannot be parsed (1-based line number).
    Parse {
        line: usize,
        msg: String,
    },
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    predicted: bool,
}

#[derive(Serialize, Deserialize)]
struct LabelArchive {
    format: String,
    version: u32,
    fingerprint: String,
    frame_count: u32,
    labels: Vec<Entry>,
}

/// Previous labels of the frames one undo step restores.
type UndoStep = Vec<(u32, Label, bool)>;

#[derive(Debug, Clone)]
pub struct LabelStore {
    frame_count: u32,
    labels: BTreeMap<u32, Label>,
    predicted: BTreeSet<u32>,
    undo: Vec<UndoStep>,
}

impl LabelStore {
    pub fn new(frame_count: u32) -> LabelStore {
        LabelStore { frame_count, labels: BTreeMap::new(), predicted: BTreeSet::new(), undo: Vec::new() }
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Portable snapshot: animation indices and prediction provenance are preserved.
    pub fn export_json(&self, fingerprint: &str) -> String {
        let archive = LabelArchive {
            format: "firecrackle-labels".into(), version: 2,
            fingerprint: fingerprint.into(), frame_count: self.frame_count,
            labels: self.iter().into_iter().map(|(frame, label)| Entry {
                version: 2, frame, label, predicted: self.is_predicted(frame),
            }).collect(),
        };
        serde_json::to_string_pretty(&archive).expect("plain label archive")
    }

    /// Validate the entire snapshot before merging; the whole import is one undo step.
    pub fn import_json(&mut self, text: &str, fingerprint: &str) -> Result<Vec<u32>, LabelError> {
        let bad = |msg: &str| LabelError::Parse { line: 0, msg: msg.into() };
        let archive: LabelArchive = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| bad(&format!("Invalid label file: {e}")))?;
        if archive.format != "firecrackle-labels" || archive.version != 2 {
            return Err(bad("Unsupported label export format or version"));
        }
        if archive.frame_count != self.frame_count || archive.fingerprint != fingerprint {
            return Err(bad("These labels belong to a different video. Open the matching video first."));
        }
        let mut seen = BTreeSet::new();
        for entry in &archive.labels {
            self.check_frame(entry.frame)?;
            entry.label.validate()?;
            if entry.version != 2 || !seen.insert(entry.frame) || (entry.predicted && !entry.label.is_fire()) {
                return Err(bad("Invalid or duplicate label entry"));
            }
        }
        let mut undo = Vec::new();
        for entry in archive.labels {
            let previous = self.get(entry.frame);
            let predicted = self.is_predicted(entry.frame);
            if (previous, predicted) == (entry.label, entry.predicted) { continue; }
            undo.push((entry.frame, previous, predicted));
            self.put(entry.frame, entry.label);
            if entry.predicted { self.predicted.insert(entry.frame); }
        }
        let mut frames: Vec<_> = undo.iter().map(|(frame, _, _)| *frame).collect();
        frames.sort_unstable();
        if !undo.is_empty() { self.undo.push(undo); }
        Ok(frames)
    }

    /// Label of a frame; `Label::Unset` when none (also for out-of-range frames).
    pub fn get(&self, frame: u32) -> Label {
        self.labels.get(&frame).copied().unwrap_or(Label::Unset)
    }

    pub fn is_predicted(&self, frame: u32) -> bool {
        self.predicted.contains(&frame)
    }

    /// Persist both value and source; accepted guesses must not become calibration evidence on restart.
    pub fn log_line(&self, frame: u32) -> String {
        serde_json::to_string(&Entry { version: 2, frame, label: self.get(frame), predicted: self.is_predicted(frame) }).expect("plain log entry")
    }

    fn check_frame(&self, frame: u32) -> Result<(), LabelError> {
        if frame < self.frame_count {
            Ok(())
        } else {
            Err(LabelError::FrameOutOfRange { frame, count: self.frame_count })
        }
    }

    fn put(&mut self, frame: u32, label: Label) {
        self.predicted.remove(&frame);
        if label == Label::Unset {
            self.labels.remove(&frame);
        } else {
            self.labels.insert(frame, label);
        }
    }

    /// Set a label. `Ok(true)` if it changed something (and pushed one undo entry),
    /// `Ok(false)` if it equalled the current label. `Label::Unset` clears.
    pub fn set(&mut self, frame: u32, label: Label) -> Result<bool, LabelError> {
        self.set_with_source(frame, label, false)
    }

    pub fn set_predicted(&mut self, frame: u32, label: Label) -> Result<bool, LabelError> {
        self.set_with_source(frame, label, true)
    }

    fn set_with_source(&mut self, frame: u32, label: Label, predicted: bool) -> Result<bool, LabelError> {
        label.validate()?;
        self.check_frame(frame)?;
        let prev = self.get(frame);
        let was_predicted = self.is_predicted(frame);
        let predicted = predicted && label.is_fire();
        if prev == label && was_predicted == predicted {
            return Ok(false);
        }
        self.undo.push(vec![(frame, prev, was_predicted)]);
        self.put(frame, label);
        if predicted {
            self.predicted.insert(frame);
        }
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
        for (frame, prev, predicted) in step {
            self.put(frame, prev);
            if predicted {
                self.predicted.insert(frame);
            }
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
            if prev != label || self.is_predicted(f) {
                step.push((f, prev, self.is_predicted(f)));
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
        let lines: Vec<(usize, &str)> = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim())).filter(|(_, l)| !l.is_empty()).collect();
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
            if entry.predicted && entry.label.is_fire() {
                store.predicted.insert(entry.frame);
            }
        }
        Ok(Replay { store, torn_tail })
    }
}

/// One versioned animation-index log entry, without a trailing newline.
pub fn log_line(frame: u32, label: Label) -> String {
    serde_json::to_string(&Entry { version: 2, frame, label, predicted: false }).expect("a plain struct always serialises")
}

/// Clamp a (possibly negative / past-the-end) frame index into `0..count`. `None` when `count == 0`.
pub fn clamp_frame(frame: i64, count: u32) -> Option<u32> {
    if count == 0 {
        None
    } else {
        Some(frame.clamp(0, count as i64 - 1) as u32)
    }
}
