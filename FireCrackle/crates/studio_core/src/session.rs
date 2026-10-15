//! The Label tab's brain: current frame, pick state, typed rows, suggestions, advance rules, undo and the
//! autosave queue, as one pure object. The UI calls one method per user action.

use crate::config::Settings;
use crate::label::{log_line, Label, LabelError, LabelStore};
use crate::phase::{analyze, Analysis, Flag, FrameObs, STEPS};
use crate::project::Project;
use serde::Serialize;

/// Result of the digit buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TyperResult {
    /// Waiting for a second digit (the first digit could start a two-digit row).
    Pending(u8),
    Commit(u8),
    /// Not a valid row (e.g. 35, or not a digit).
    Rejected,
}

/// Turns typed digits into a row 0..31: `1` waits for a second digit, `4`..`9` commit at once, `12` commits 12,
/// `35` is rejected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowTyper {
    pending: Option<u8>,
}

impl RowTyper {
    pub fn new() -> RowTyper {
        RowTyper { pending: None }
    }

    pub fn push(&mut self, digit: u8) -> TyperResult {
        if digit > 9 {
            self.pending = None;
            return TyperResult::Rejected;
        }
        match self.pending.take() {
            None if digit >= 4 => TyperResult::Commit(digit), // 40 and up are not rows
            None => {
                self.pending = Some(digit);
                TyperResult::Pending(digit)
            }
            Some(first) => {
                let row = first * 10 + digit;
                if row < STEPS {
                    TyperResult::Commit(row)
                } else {
                    TyperResult::Rejected
                }
            }
        }
    }

    /// Commit a pending single digit (Enter, or the timeout).
    pub fn flush(&mut self) -> Option<u8> {
        self.pending.take()
    }

    pub fn clear(&mut self) {
        self.pending = None;
    }

    pub fn pending(&self) -> Option<u8> {
        self.pending
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Frames(i64),
    To(u32),
    Home,
    End,
    NextUnlabelled,
    PrevUnlabelled,
    NextFlag,
    PrevFlag,
}

/// Which sprite the next typed row refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PickState {
    pub sheet: u8,
    pub mirrored: bool,
}

/// A label was set at `frame`; the playhead is now at `now_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub frame: u32,
    pub now_at: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeOutcome {
    Pending(u8),
    Committed(Applied),
    Rejected,
}

/// The likely row for the current frame, from the nearest labelled fire frame (E7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    pub sheet: u8,
    pub mirrored: bool,
    pub row: u8,
    /// The other possible row (the sprite advanced one tick more or less): the tick boundary falls near here.
    pub alt_row: Option<u8>,
    pub from_frame: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    Label(LabelError),
    /// The project and the label store disagree on the number of frames.
    FrameCountMismatch { project: usize, store: u32 },
    NoSuggestion,
}

impl From<LabelError> for SessionError {
    fn from(e: LabelError) -> SessionError {
        SessionError::Label(e)
    }
}

/// Distance from a whole number of ticks below which the advance is treated as certain.
const WHOLE_TICK_EPS: f64 = 1e-6;

pub struct Session {
    project: Project,
    store: LabelStore,
    settings: Settings,
    current: u32,
    pick: PickState,
    typer: RowTyper,
    log: Vec<String>,
    analysis: Option<Analysis>,
}

impl Session {
    /// The playhead starts on the first unlabelled frame (frame 0 when everything is labelled).
    pub fn new(project: Project, store: LabelStore, settings: Settings) -> Result<Session, SessionError> {
        if project.frames.len() != store.frame_count() as usize {
            return Err(SessionError::FrameCountMismatch { project: project.frames.len(), store: store.frame_count() });
        }
        let current = if store.get(0) == Label::Unset { 0 } else { store.next_unlabelled(0, true).unwrap_or(0) };
        Ok(Session {
            project,
            store,
            settings,
            current,
            pick: PickState { sheet: 1, mirrored: false },
            typer: RowTyper::new(),
            log: Vec::new(),
            analysis: None,
        })
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn store(&self) -> &LabelStore {
        &self.store
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.analysis = None; // the phase thresholds may have changed
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    fn last(&self) -> u32 {
        self.store.frame_count().saturating_sub(1)
    }

    /// Label of the current frame.
    pub fn label(&self) -> Label {
        self.store.get(self.current)
    }

    pub fn pick(&self) -> PickState {
        self.pick
    }

    /// The first digit of a two-digit row typed so far, if any.
    pub fn typed_pending(&self) -> Option<u8> {
        self.typer.pending()
    }

    pub fn go(&mut self, m: Move) -> u32 {
        self.typer.clear();
        let (cur, last) = (self.current, self.last());
        self.current = match m {
            Move::Frames(d) => (cur as i64).saturating_add(d).clamp(0, last as i64) as u32,
            Move::To(n) => n.min(last),
            Move::Home => 0,
            Move::End => last,
            Move::NextUnlabelled => self.store.next_unlabelled(cur, true).unwrap_or(cur),
            Move::PrevUnlabelled => self.store.next_unlabelled(cur, false).unwrap_or(cur),
            Move::NextFlag => self.analysis().flags.iter().map(|f| f.frame).find(|&f| f > cur).unwrap_or(cur),
            Move::PrevFlag => self.analysis().flags.iter().rev().map(|f| f.frame).find(|&f| f < cur).unwrap_or(cur),
        };
        self.current
    }

    pub fn toggle_sheet(&mut self) {
        self.pick.sheet = 1 - self.pick.sheet.min(1);
    }

    pub fn toggle_mirror(&mut self) {
        self.pick.mirrored = !self.pick.mirrored;
    }

    pub fn set_pick(&mut self, sheet: u8, mirrored: bool) {
        self.pick = PickState { sheet, mirrored };
    }

    /// Set the label of the current frame, then advance per the settings. `Unset` clears and stays.
    pub fn apply(&mut self, label: Label) -> Result<Applied, SessionError> {
        self.typer.clear();
        let frame = self.current;
        if self.store.set(frame, label)? {
            self.log.push(log_line(frame, label));
            self.analysis = None;
        }
        let last = self.last();
        let now_at = if label == Label::Unset {
            frame
        } else if self.settings.label.advance_to_unlabelled {
            self.store.next_unlabelled(frame, true).unwrap_or(frame)
        } else {
            (frame as u64 + self.settings.label.advance_frames as u64).min(last as u64) as u32
        };
        self.current = now_at;
        Ok(Applied { frame, now_at })
    }

    /// Label the current frame as the picked sheet/mirror with `row`.
    pub fn pick_row(&mut self, row: u8) -> Result<Applied, SessionError> {
        self.apply(Label::Fire { sheet: self.pick.sheet, row, mirrored: self.pick.mirrored })
    }

    /// A click on a picker cell: sets the pick state and labels the frame.
    pub fn click_cell(&mut self, sheet: u8, row: u8, mirrored: bool) -> Result<Applied, SessionError> {
        let label = Label::fire(sheet, row, mirrored)?; // validated before the pick state changes
        self.pick = PickState { sheet, mirrored };
        self.apply(label)
    }

    pub fn type_digit(&mut self, digit: u8) -> Result<TypeOutcome, SessionError> {
        Ok(match self.typer.push(digit) {
            TyperResult::Pending(d) => TypeOutcome::Pending(d),
            TyperResult::Commit(row) => TypeOutcome::Committed(self.pick_row(row)?),
            TyperResult::Rejected => TypeOutcome::Rejected,
        })
    }

    /// Commit a pending typed digit (Enter or the timeout).
    pub fn flush_typed(&mut self) -> Result<Option<Applied>, SessionError> {
        match self.typer.flush() {
            Some(row) => Ok(Some(self.pick_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn cancel_typed(&mut self) {
        self.typer.clear();
    }

    pub fn suggestion(&self) -> Option<Suggestion> {
        let max_seconds = self.settings.label.suggest_max_seconds;
        if max_seconds <= 0.0 {
            return None;
        }
        // nearest labelled fire frame; ties go to the earlier one (iteration is ascending, `<` keeps it)
        let mut best: Option<(u32, u32, u8, u8, bool)> = None; // (distance, frame, sheet, row, mirrored)
        for (frame, label) in self.store.iter() {
            let Label::Fire { sheet, row, mirrored } = label else { continue };
            if frame == self.current {
                continue;
            }
            let dist = frame.abs_diff(self.current);
            if best.is_none_or(|b| dist < b.0) {
                best = Some((dist, frame, sheet, row, mirrored));
            }
        }
        let (_, from_frame, sheet, row, mirrored) = best?;

        let dt = self.project.frames[self.current as usize].pts - self.project.frames[from_frame as usize].pts;
        if dt.abs() > max_seconds {
            return None;
        }
        // x ticks of video time have passed; the sprite advanced floor(x) or floor(x) + 1 ticks, the latter
        // with probability frac(x) (E7). Likelier first, the other as the alternative.
        let x = self.settings.phase.tick_rate * dt;
        let whole = x.round();
        let (advance, alt) = if (x - whole).abs() < WHOLE_TICK_EPS {
            (whole as i64, None)
        } else {
            let floor = x.floor();
            if x - floor >= 0.5 {
                (floor as i64 + 1, Some(floor as i64))
            } else {
                (floor as i64, Some(floor as i64 + 1))
            }
        };
        let to_row = |a: i64| (row as i64 + a).rem_euclid(STEPS as i64) as u8;
        Some(Suggestion { sheet, mirrored, row: to_row(advance), alt_row: alt.map(to_row), from_frame })
    }

    pub fn accept_suggestion(&mut self) -> Result<Applied, SessionError> {
        let s = self.suggestion().ok_or(SessionError::NoSuggestion)?;
        self.pick = PickState { sheet: s.sheet, mirrored: s.mirrored };
        self.apply(Label::Fire { sheet: s.sheet, row: s.row, mirrored: s.mirrored })
    }

    pub fn fill_range(&mut self, start: u32, end: u32, label: Label) -> Result<usize, SessionError> {
        let in_range = start <= end && end < self.store.frame_count();
        let before: Vec<Label> = if in_range { (start..=end).map(|f| self.store.get(f)).collect() } else { Vec::new() };
        let changed = self.store.fill_range(start, end, label)?;
        if changed > 0 {
            for (i, frame) in (start..=end).enumerate() {
                if before[i] != label {
                    self.log.push(log_line(frame, label));
                }
            }
            self.analysis = None;
        }
        Ok(changed)
    }

    /// Undo the last change; the playhead moves to the first frame it restored.
    pub fn undo(&mut self) -> bool {
        match self.store.undo_step() {
            Some(frames) => {
                for &f in &frames {
                    self.log.push(log_line(f, self.store.get(f)));
                }
                self.current = frames[0];
                self.typer.clear();
                self.analysis = None;
                true
            }
            None => false,
        }
    }

    /// Phase analysis of the current labels (cached until the labels change).
    pub fn analysis(&mut self) -> &Analysis {
        if self.analysis.is_none() {
            let obs: Vec<FrameObs> = self
                .store
                .iter()
                .into_iter()
                .filter(|(_, l)| l.is_fire())
                .map(|(frame, label)| FrameObs { frame, pts: self.project.frames[frame as usize].pts, label })
                .collect();
            self.analysis = Some(analyze(&obs, &self.settings.phase).expect("labels and project timestamps are valid"));
        }
        self.analysis.as_ref().expect("just computed")
    }

    pub fn flags(&mut self) -> &[Flag] {
        &self.analysis().flags
    }

    /// Log lines for every change since the last call (undo included), ready for `AutosaveLog::append`.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }

    /// Put lines back in front of the queue after a failed write.
    pub fn requeue_log(&mut self, lines: Vec<String>) {
        let mut v = lines;
        v.append(&mut self.log);
        self.log = v;
    }
}
