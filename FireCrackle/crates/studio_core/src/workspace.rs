//! Everything the app needs behind one handle: the project folder, the session, the autosave log and the frame
//! service. The Tauri layer is a thin shim over this; the UI sends a `UiAction` and gets a `View` back.

use crate::autosave::{AutosaveError, AutosaveLog};
use crate::config::{DecodeConfig, SettingError, SettingInfo, Settings};
use crate::decode_plan::{plan, DecodeJob, FrameCache, PlanError};
use crate::frames::{decode_job, DecodeError, DecodedFrame};
use crate::import::{import_video, ImportError, Tools};
use crate::index::FrameIndex;
use crate::label::Label;
use crate::lane::{buckets, Bucket};
use crate::chart::decimate;
use crate::phase::{Flag, Point, Run};
use crate::project::{Project, ProjectError};
use crate::session::{Move, PickState, Session, SessionError, Suggestion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// ============================================================ frame service

/// Where decoded frames come from. The real one is ffmpeg; tests use a fake that counts calls.
pub trait FrameSource: Send + Sync {
    fn decode(&self, job: &DecodeJob, cancel: &AtomicBool) -> Result<Vec<DecodedFrame>, DecodeError>;
}

/// The ffmpeg-backed source for one video.
pub struct FfmpegSource {
    pub tools: Tools,
    pub video: PathBuf,
    pub index: Arc<FrameIndex>,
    /// Shared with the workspace so a changed setting reaches the next decode.
    pub cfg: Arc<Mutex<DecodeConfig>>,
}

impl FrameSource for FfmpegSource {
    fn decode(&self, job: &DecodeJob, cancel: &AtomicBool) -> Result<Vec<DecodedFrame>, DecodeError> {
        let cfg = *self.cfg.lock().expect("decode settings lock");
        decode_job(&self.tools, &self.video, &self.index, job, &cfg, cancel)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameError {
    /// The frame is not in the video.
    OutOfRange(u32),
    Decode(DecodeError),
    Plan(PlanError),
}

struct State {
    cache: FrameCache,
    data: HashMap<u32, Arc<Vec<u8>>>,
}

struct Inner {
    index: Arc<FrameIndex>,
    source: Arc<dyn FrameSource>,
    cfg: Mutex<DecodeConfig>,
    /// Whether the capacity follows `cfg.cache_mb` (false for an explicit capacity).
    capacity_from_cfg: bool,
    state: Mutex<State>,
    busy: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
    never_cancelled: AtomicBool,
}

impl Inner {
    /// Store decoded frames (evicted ones leave memory too). `last` is inserted last so it is the most recently
    /// used and survives even a cache smaller than the window.
    fn insert(&self, frames: Vec<DecodedFrame>, last: Option<u32>) -> Option<Arc<Vec<u8>>> {
        let mut st = self.state.lock().expect("frame cache lock");
        let mut keep: Option<Arc<Vec<u8>>> = None;
        let (mut first_pass, mut second_pass): (Vec<DecodedFrame>, Vec<DecodedFrame>) = (Vec::new(), Vec::new());
        for f in frames {
            if Some(f.frame) == last {
                second_pass.push(f);
            } else {
                first_pass.push(f);
            }
        }
        first_pass.extend(second_pass);
        for f in first_pass {
            let bytes = Arc::new(f.jpeg);
            if Some(f.frame) == last {
                keep = Some(bytes.clone());
            }
            for gone in st.cache.insert(f.frame, bytes.len() as u64) {
                st.data.remove(&gone);
            }
            st.data.insert(f.frame, bytes);
        }
        keep
    }
}

/// Decoded frames in memory around the playhead: a byte cache driven by the decode planner, with a background
/// prefetch that follows the planner's hysteresis.
pub struct FrameService {
    inner: Arc<Inner>,
}

impl FrameService {
    /// Cache capacity comes from `cfg.cache_mb`.
    pub fn new(index: Arc<FrameIndex>, source: Arc<dyn FrameSource>, cfg: DecodeConfig) -> FrameService {
        let cap = cfg.cache_mb as u64 * 1024 * 1024;
        FrameService::build(index, source, cfg, cap, true)
    }

    /// Like `new` with an explicit capacity in bytes (tests use tiny caches).
    pub fn with_capacity(index: Arc<FrameIndex>, source: Arc<dyn FrameSource>, cfg: DecodeConfig, cap_bytes: u64) -> FrameService {
        FrameService::build(index, source, cfg, cap_bytes, false)
    }

    fn build(index: Arc<FrameIndex>, source: Arc<dyn FrameSource>, cfg: DecodeConfig, cap: u64, from_cfg: bool) -> FrameService {
        FrameService {
            inner: Arc::new(Inner {
                index,
                source,
                cfg: Mutex::new(cfg),
                capacity_from_cfg: from_cfg,
                state: Mutex::new(State { cache: FrameCache::new(cap), data: HashMap::new() }),
                busy: AtomicBool::new(false),
                worker: Mutex::new(None),
                never_cancelled: AtomicBool::new(false),
            }),
        }
    }

    /// The JPEG of `frame`. A miss decodes the planned run and blocks; a hit returns at once and may start a
    /// background prefetch. A failed decode is an error and leaves the cache unchanged.
    pub fn get(&self, frame: u32) -> Result<Arc<Vec<u8>>, FrameError> {
        let inner = &self.inner;
        if frame as usize >= inner.index.frames.len() {
            return Err(FrameError::OutOfRange(frame));
        }
        let cfg = *inner.cfg.lock().expect("decode config lock");
        loop {
            let plan = {
                let st = inner.state.lock().expect("frame cache lock");
                plan(&inner.index, &st.cache, frame, &cfg).map_err(FrameError::Plan)?
            };
            if plan.want_cached {
                let hit = {
                    let mut st = inner.state.lock().expect("frame cache lock");
                    let bytes = st.data.get(&frame).cloned();
                    if bytes.is_some() {
                        st.cache.touch(frame);
                    }
                    bytes
                };
                if let Some(bytes) = hit {
                    if plan.job.is_some() {
                        self.prefetch(frame);
                    }
                    return Ok(bytes);
                }
                continue; // evicted between planning and reading: plan again
            }
            let job = plan.job.expect("a missing frame always has a job");
            let decoded = inner.source.decode(&job, &inner.never_cancelled).map_err(FrameError::Decode)?;
            return inner
                .insert(decoded, Some(frame))
                .ok_or_else(|| FrameError::Decode(DecodeError::BadStream(format!("the decode did not contain frame {frame}"))));
        }
    }

    /// Top the window up around `around` on a background thread (no-op while one is running or nothing is due).
    pub fn prefetch(&self, around: u32) {
        if self.inner.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let inner = self.inner.clone();
        let handle = std::thread::spawn(move || {
            let cfg = *inner.cfg.lock().expect("decode config lock");
            let job = {
                let st = inner.state.lock().expect("frame cache lock");
                plan(&inner.index, &st.cache, around, &cfg).ok().and_then(|p| p.job)
            };
            if let Some(job) = job {
                // a failed prefetch is not an error: the next request plans again
                if let Ok(frames) = inner.source.decode(&job, &inner.never_cancelled) {
                    inner.insert(frames, None);
                }
            }
            inner.busy.store(false, Ordering::SeqCst);
        });
        *self.inner.worker.lock().expect("worker lock") = Some(handle);
    }

    /// Block until no background prefetch is running.
    pub fn wait_idle(&self) {
        let handle = self.inner.worker.lock().expect("worker lock").take();
        if let Some(h) = handle {
            let _ = h.join();
        }
    }

    /// Cached frames, ascending.
    pub fn cached_frames(&self) -> Vec<u32> {
        self.inner.state.lock().expect("frame cache lock").cache.frames()
    }

    pub fn used_bytes(&self) -> u64 {
        self.inner.state.lock().expect("frame cache lock").cache.used_bytes()
    }

    /// New window sizes and capacity apply from the next request on (a smaller cache evicts at once).
    pub fn set_config(&self, cfg: DecodeConfig) {
        *self.inner.cfg.lock().expect("decode config lock") = cfg;
        if self.inner.capacity_from_cfg {
            let mut st = self.inner.state.lock().expect("frame cache lock");
            for gone in st.cache.set_capacity(cfg.cache_mb as u64 * 1024 * 1024) {
                st.data.remove(&gone);
            }
        }
    }
}

// ============================================================== workspace

/// One user action from the Label tab.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiAction {
    Step { frames: i64 },
    BigStep { dir: i64 },
    Home,
    End,
    GoTo { frame: u32 },
    NextUnlabelled { dir: i64 },
    NextFlag { dir: i64 },
    ToggleSheet,
    ToggleMirror,
    Digit { digit: u8 },
    FlushTyped,
    CancelTyping,
    AcceptSuggestion,
    Clear,
    NoFire,
    Skip,
    Undo,
    ClickCell { col: u32, row: u32 },
    FillRange { start: u32, end: u32, label: Label },
}

/// Everything the UI draws, after any action.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    pub current: u32,
    pub frame_count: u32,
    /// Presentation time of the current frame, seconds.
    pub pts: f64,
    pub fps: f64,
    pub label: Label,
    pub pick: PickState,
    /// First digit of a two-digit row typed so far.
    pub typed_pending: Option<u8>,
    pub suggestion: Option<Suggestion>,
    pub labelled_count: usize,
    pub flag_count: usize,
    /// Things worth telling the user: import warnings, a repaired log, unreadable settings.
    pub warnings: Vec<String>,
    /// The last autosave failed; the changes are queued and retried with the next action.
    pub save_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkspaceError {
    Import(ImportError),
    Project(ProjectError),
    Autosave(AutosaveError),
    Session(SessionError),
    Frame(FrameError),
    Setting(SettingError),
    Io(String),
    /// An action the UI should not have sent (cell outside the grid, ...).
    BadAction(String),
}

/// The picker columns, left to right: (sheet, mirrored). Must match `COLUMNS` in `src/lib/grid.ts`.
const COLUMNS: [(u8, bool); 4] = [(0, false), (1, false), (0, true), (1, true)];

/// What the Phase tab draws.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhaseChart {
    pub points: Vec<Point>,
    pub runs: Vec<Run>,
    pub flags: Vec<Flag>,
    pub tick_rate: f64,
    /// Width of the arc one steady run fits in (the `phase.window_ticks` setting), for the chart's bands.
    pub window_ticks: f64,
}

pub struct Workspace {
    dir: PathBuf,
    session: Session,
    log: AutosaveLog,
    frames: FrameService,
    /// Shared with the ffmpeg source (absent when the source is supplied by a test).
    decode_cfg: Option<Arc<Mutex<DecodeConfig>>>,
    warnings: Vec<String>,
    save_error: Option<String>,
}

fn io_err(e: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io(e.to_string())
}

/// Write a file atomically (temp file in the same folder, then rename).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), WorkspaceError> {
    let mut name = path.file_name().ok_or_else(|| WorkspaceError::Io("no file name".into()))?.to_os_string();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    let result = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(io_err)
}

impl Workspace {
    /// Open (or create) the project for `video`: `<video>.firecrackle/` holds `project.json`, `labels.jsonl` and
    /// `settings.json`. An existing project is loaded instead of re-imported; a replaced video or a label log of
    /// another video is refused.
    pub fn open(tools: &Tools, video: &Path, defaults: &Settings) -> Result<Workspace, WorkspaceError> {
        let mut name = video.file_name().ok_or_else(|| WorkspaceError::Io("the video path has no file name".into()))?.to_os_string();
        name.push(".firecrackle");
        let dir = video.with_file_name(name);
        std::fs::create_dir_all(&dir).map_err(io_err)?;
        let project_path = dir.join("project.json");

        let project = if project_path.is_file() {
            let mut p = Project::load(&project_path).map_err(WorkspaceError::Project)?;
            let here = video.to_string_lossy().into_owned();
            let moved = p.video_path != here;
            p.video_path = here; // the project folder travels with the video
            p.verify_video().map_err(WorkspaceError::Project)?;
            if moved {
                p.save(&project_path).map_err(WorkspaceError::Project)?;
            }
            p
        } else {
            let p = import_video(tools, video, defaults).map_err(WorkspaceError::Import)?;
            p.save(&project_path).map_err(WorkspaceError::Project)?;
            p
        };

        let decode_cfg = Arc::new(Mutex::new(defaults.decode));
        let index = Arc::new(project.frame_index());
        let source = Arc::new(FfmpegSource {
            tools: tools.clone(),
            video: video.to_path_buf(),
            index,
            cfg: decode_cfg.clone(),
        });
        Workspace::build(&dir, project, defaults, source, Some(decode_cfg))
    }

    /// The same with the project and the frame source supplied (tests; the real `open` calls the same code).
    pub fn from_parts(
        dir: &Path,
        project: Project,
        defaults: &Settings,
        source: Arc<dyn FrameSource>,
    ) -> Result<Workspace, WorkspaceError> {
        Workspace::build(dir, project, defaults, source, None)
    }

    fn build(
        dir: &Path,
        project: Project,
        defaults: &Settings,
        source: Arc<dyn FrameSource>,
        decode_cfg: Option<Arc<Mutex<DecodeConfig>>>,
    ) -> Result<Workspace, WorkspaceError> {
        std::fs::create_dir_all(dir).map_err(io_err)?;
        let mut warnings = project.warnings.clone();

        let settings = match std::fs::read(dir.join("settings.json")) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    warnings.push(format!("settings.json could not be read ({e}); using the default settings."));
                    *defaults
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => *defaults,
            Err(e) => {
                warnings.push(format!("settings.json could not be read ({e}); using the default settings."));
                *defaults
            }
        };
        if let Some(h) = &decode_cfg {
            *h.lock().expect("decode settings lock") = settings.decode;
        }

        let (log, store, report) =
            AutosaveLog::open(&dir.join("labels.jsonl"), project.frames.len() as u32, &settings.autosave)
                .map_err(WorkspaceError::Autosave)?;
        if report.torn_tail_repaired {
            warnings.push(
                "The label log ended in a partial line (a crash while saving?). The partial line was removed; \\
                 every complete label was kept: the log was repaired."
                    .to_string(),
            );
        }

        let index = Arc::new(project.frame_index());
        let frames = FrameService::new(index, source, settings.decode);
        let session = Session::new(project, store, settings).map_err(WorkspaceError::Session)?;
        Ok(Workspace { dir: dir.to_path_buf(), session, log, frames, decode_cfg, warnings, save_error: None })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn view(&mut self) -> View {
        let current = self.session.current();
        let project = self.session.project();
        let pts = project.frames[current as usize].pts;
        let index_fps = project.index.fps;
        let fps = if index_fps > 0.0 {
            index_fps
        } else {
            project.meta.fps_num as f64 / project.meta.fps_den.max(1) as f64
        };
        let frame_count = self.session.store().frame_count();
        let flag_count = self.session.flags().len();
        View {
            current,
            frame_count,
            pts,
            fps,
            label: self.session.label(),
            pick: self.session.pick(),
            typed_pending: self.session.typed_pending(),
            suggestion: self.session.suggestion(),
            labelled_count: self.session.store().labelled_count(),
            flag_count,
            warnings: self.warnings.clone(),
            save_error: self.save_error.clone(),
        }
    }

    /// Execute an action, flush the autosave queue, return the new view.
    pub fn apply(&mut self, action: UiAction) -> Result<View, WorkspaceError> {
        let bad = |why: &str| WorkspaceError::BadAction(why.to_string());
        let session_err = WorkspaceError::Session;
        let big = self.session.settings().label.big_step_frames as i64;
        let direction = |dir: i64, what: &str| -> Result<bool, WorkspaceError> {
            match dir {
                d if d > 0 => Ok(true),
                d if d < 0 => Ok(false),
                _ => Err(WorkspaceError::BadAction(format!("{what} needs a direction (+1 or -1)"))),
            }
        };
        match action {
            UiAction::Step { frames } => {
                self.session.go(Move::Frames(frames));
            }
            UiAction::BigStep { dir } => {
                let forward = direction(dir, "a big step")?;
                self.session.go(Move::Frames(if forward { big } else { -big }));
            }
            UiAction::Home => {
                self.session.go(Move::Home);
            }
            UiAction::End => {
                self.session.go(Move::End);
            }
            UiAction::GoTo { frame } => {
                self.session.go(Move::To(frame));
            }
            UiAction::NextUnlabelled { dir } => {
                let forward = direction(dir, "next unlabelled")?;
                self.session.go(if forward { Move::NextUnlabelled } else { Move::PrevUnlabelled });
            }
            UiAction::NextFlag { dir } => {
                let forward = direction(dir, "next flag")?;
                self.session.go(if forward { Move::NextFlag } else { Move::PrevFlag });
            }
            UiAction::ToggleSheet => self.session.toggle_sheet(),
            UiAction::ToggleMirror => self.session.toggle_mirror(),
            UiAction::Digit { digit } => {
                self.session.type_digit(digit).map_err(session_err)?;
            }
            UiAction::FlushTyped => {
                self.session.flush_typed().map_err(session_err)?;
            }
            UiAction::CancelTyping => self.session.cancel_typed(),
            UiAction::AcceptSuggestion => {
                self.session.accept_suggestion().map_err(session_err)?;
            }
            UiAction::Clear => {
                self.session.apply(Label::Unset).map_err(session_err)?;
            }
            UiAction::NoFire => {
                self.session.apply(Label::NoFire).map_err(session_err)?;
            }
            UiAction::Skip => {
                self.session.apply(Label::Skip).map_err(session_err)?;
            }
            UiAction::Undo => {
                self.session.undo();
            }
            UiAction::ClickCell { col, row } => {
                let (sheet, mirrored) = *COLUMNS.get(col as usize).ok_or_else(|| bad("the picker has only 4 columns"))?;
                let row = u8::try_from(row).map_err(|_| bad("row out of range"))?;
                self.session.click_cell(sheet, row, mirrored).map_err(session_err)?;
            }
            UiAction::FillRange { start, end, label } => {
                self.session.fill_range(start, end, label).map_err(session_err)?;
            }
        }
        self.flush();
        Ok(self.view())
    }

    /// Write the queued log lines; a failure keeps them queued and is reported, never fatal.
    fn flush(&mut self) {
        let lines = self.session.take_log();
        if !lines.is_empty() {
            match self.log.append(&lines) {
                Ok(()) => self.save_error = None,
                Err(e) => {
                    self.session.requeue_log(lines);
                    self.save_error = Some(format!("Could not save the labels: {e:?}. The changes are kept and will be retried."));
                    return;
                }
            }
        }
        if let Err(e) = self.log.compact_if_needed(self.session.store()) {
            self.save_error = Some(format!("Could not compact the label log: {e:?}"));
        }
    }

    pub fn frame_jpeg(&self, frame: u32) -> Result<Arc<Vec<u8>>, WorkspaceError> {
        self.frames.get(frame).map_err(WorkspaceError::Frame)
    }

    pub fn timeline(&mut self, start: u32, end: u32, n: usize) -> Vec<Bucket> {
        let flags = self.session.flags().to_vec();
        buckets(self.session.store(), &flags, start, end, n)
    }

    pub fn flags(&mut self) -> Vec<Flag> {
        self.session.flags().to_vec()
    }

    /// Everything the Phase tab draws: the phase series (reduced to at most `max_points`), the runs and the flags.
    pub fn phase_chart(&mut self, max_points: usize) -> PhaseChart {
        let tick_rate = self.session.settings().phase.tick_rate;
        let window_ticks = self.session.settings().phase.window_ticks;
        let a = self.session.analysis();
        PhaseChart {
            points: decimate(&a.points, max_points),
            runs: a.runs.clone(),
            flags: a.flags.clone(),
            tick_rate,
            window_ticks,
        }
    }

    pub fn settings(&self) -> Settings {
        *self.session.settings()
    }

    pub fn describe_settings(&self) -> Vec<SettingInfo> {
        self.session.settings().describe()
    }

    /// Validate, apply and persist one setting.
    pub fn set_setting(&mut self, key: &str, value: f64) -> Result<View, WorkspaceError> {
        let mut s = self.settings();
        s.set(key, value).map_err(WorkspaceError::Setting)?;
        let json = serde_json::to_vec_pretty(&s).map_err(|e| WorkspaceError::Io(e.to_string()))?;
        write_atomic(&self.dir.join("settings.json"), &json)?;
        self.session.set_settings(s);
        self.frames.set_config(s.decode);
        if let Some(h) = &self.decode_cfg {
            *h.lock().expect("decode settings lock") = s.decode;
        }
        self.log.set_config(&s.autosave);
        Ok(self.view())
    }

    pub fn wait_idle(&self) {
        self.frames.wait_idle();
    }
}
