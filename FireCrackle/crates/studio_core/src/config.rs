//! Every tunable threshold in one place, each with a default, a valid range and a plain-language
//! explanation (`Settings::describe`), so a settings screen can be generated from it and nothing is a
//! hidden constant. Older settings files keep working: missing fields fall back to their defaults.
//!

use crate::phase::Config as PhaseConfig;
use serde::{Deserialize, Serialize};

/// How the frame index judges timing regularity.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexConfig {
    /// An interval between frames counts as irregular when it differs from the median interval by more
    /// than this fraction of the median.
    pub irregular_deviation: f64,
    /// The video is called variable frame rate when more than this fraction of intervals is irregular.
    pub vfr_fraction: f64,
}

impl Default for IndexConfig {
    fn default() -> IndexConfig {
        IndexConfig { irregular_deviation: 0.5, vfr_fraction: 0.01 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// How many bytes at the start and at the end of the video go into its fingerprint.
    pub fingerprint_window_bytes: u64,
}

impl Default for ProjectConfig {
    fn default() -> ProjectConfig {
        ProjectConfig { fingerprint_window_bytes: 1 << 20 }
    }
}

/// How labelling behaves in the Label tab.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LabelConfig {
    /// Frames to move forward after a label is set (0 = stay on the frame).
    pub advance_frames: u32,
    /// Frames moved by the big-step keys.
    pub big_step_frames: u32,
    /// After labelling, jump to the next unlabelled frame instead of moving by `advance_frames`.
    pub advance_to_unlabelled: bool,
    /// A single typed row digit that could start a two-digit row is committed after this long.
    pub row_type_timeout_ms: u32,
    /// No row suggestion further than this many seconds from the nearest labelled fire frame.
    pub suggest_max_seconds: f64,
    /// Fastest playback with the J/L shuttle keys, as a multiple of the video's frame rate.
    pub max_play_speed: u32,
    /// Shortest time between two playback steps in milliseconds; faster playback steps several frames at once.
    pub min_play_tick_ms: u32,
}

impl Default for LabelConfig {
    fn default() -> LabelConfig {
        LabelConfig {
            advance_frames: 1,
            big_step_frames: 10,
            advance_to_unlabelled: false,
            row_type_timeout_ms: 700,
            suggest_max_seconds: 30.0,
            max_play_speed: 8,
            min_play_tick_ms: 16,
        }
    }
}

/// The decoded-frame window around the playhead.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DecodeConfig {
    /// Frames kept decoded behind the playhead.
    pub window_back: u32,
    /// Frames kept decoded ahead of the playhead.
    pub window_ahead: u32,
    /// Prefetch only when the cached frames ahead/behind fell under this fraction of the window.
    pub prefetch_fraction: f64,
    /// Decoded-frame cache size in MiB.
    pub cache_mb: u32,
    /// Decoded frames are scaled down to at most this width.
    pub max_width: u32,
    /// A decode that has to start more than this many frames before the wanted one is flagged slow.
    pub max_gop_frames: u32,
    /// The seek is aimed this many milliseconds before the keyframe's timestamp (timestamps are printed
    /// rounded, and a seek that lands a hair after the keyframe would drop it).
    pub seek_backoff_ms: f64,
    /// A decoded frame whose timestamp differs from the index by more than this fraction of the local frame
    /// interval is reported as misaligned instead of being shown.
    pub align_tolerance: f64,
    /// JPEG quality of the decoded frames, ffmpeg scale 2 (best) to 31 (worst).
    pub jpeg_quality: u32,
}

impl Default for DecodeConfig {
    fn default() -> DecodeConfig {
        DecodeConfig {
            window_back: 30,
            window_ahead: 90,
            prefetch_fraction: 0.5,
            cache_mb: 512,
            max_width: 1280,
            max_gop_frames: 600,
            seek_backoff_ms: 0.5,
            align_tolerance: 0.5,
            jpeg_quality: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutosaveConfig {
    /// The label log is rewritten compactly when it grows beyond this many lines.
    pub compact_after_lines: u32,
    /// Flush every append to disk (a crash then loses nothing, at a small cost per label).
    pub fsync: bool,
}

impl Default for AutosaveConfig {
    fn default() -> AutosaveConfig {
        AutosaveConfig { compact_after_lines: 50_000, fsync: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimelineConfig {
    /// Zoom factor per mouse-wheel notch.
    pub zoom_factor: f64,
    /// Most zoomed in: pixels per frame.
    pub max_px_per_frame: f64,
    /// Width of a timeline thumbnail in pixels.
    pub thumb_width_px: u32,
    /// JPEG quality of the thumbnails (ffmpeg MJPEG `-q:v`, 2 = best, 31 = smallest).
    pub thumb_quality: u32,
    /// Most thumbnails kept for a video; the keyframes are thinned evenly to fit.
    pub thumb_max_count: u32,
    /// Whether the thumbnail strip is generated at all.
    pub thumbs_enabled: bool,
    /// While playing, the timeline scrolls to keep the playhead at least this far from its edges.
    pub follow_margin_px: u32,
    /// Closest allowed spacing of ruler labels in pixels.
    pub ruler_spacing_px: u32,
    /// A mouse press moved less than this many pixels is a click (seek), more is a drag (pan).
    pub drag_threshold_px: u32,
}

impl Default for TimelineConfig {
    fn default() -> TimelineConfig {
        TimelineConfig {
            zoom_factor: 1.25,
            max_px_per_frame: 32.0,
            thumb_width_px: 96,
            thumb_quality: 5,
            thumb_max_count: 600,
            thumbs_enabled: true,
            follow_margin_px: 80,
            ruler_spacing_px: 90,
            drag_threshold_px: 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub phase: PhaseConfig,
    pub index: IndexConfig,
    pub project: ProjectConfig,
    pub label: LabelConfig,
    pub decode: DecodeConfig,
    pub autosave: AutosaveConfig,
    pub timeline: TimelineConfig,
}

/// One setting as a UI shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettingInfo {
    /// Stable key, `group.name`, e.g. `phase.min_run`.
    pub key: &'static str,
    pub label: &'static str,
    pub value: f64,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    /// True when only whole numbers are valid.
    pub integer: bool,
    pub explanation: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingError {
    UnknownKey(String),
    NotFinite(String),
    NotAnInteger(String),
    OutOfRange { key: String, value: f64, min: f64, max: f64 },
}

/// One row of the settings table: how to show, validate, read and write a setting.
struct Spec {
    key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    integer: bool,
    explanation: &'static str,
    get: fn(&Settings) -> f64,
    set: fn(&mut Settings, f64),
}

static SPECS: [Spec; 33] = [
    Spec {
        key: "phase.min_run",
        label: "Phase: shortest believable run (frames)",
        min: 1.0,
        max: 1_000_000.0,
        integer: true,
        explanation: "A stretch of labelled frames with the same phase that is shorter than this, and sits between \
two stretches with one identical phase, is flagged as a hard outlier (a mislabelled frame). Longer stretches are \
treated as a real phase change and flagged softly instead (a resource reload, leaving the world, or drift). \
Raise it to flag longer odd stretches as outliers; lower it to trust short runs.",
        get: |s| s.phase.min_run as f64,
        set: |s, v| s.phase.min_run = v as usize,
    },
    Spec {
        key: "phase.tick_rate",
        label: "Phase: game ticks per second of video",
        min: 1.0,
        max: 1000.0,
        integer: false,
        explanation: "How many game ticks pass per second of video time. Minecraft runs 20 per second, so 20 is \
right for real-time footage. The tick is estimated from the video timestamp, so footage that was sped up, slowed \
down or time-lapsed needs the matching value; a wrong value shows up as the phase drifting steadily and many \
shift flags. For example, half-speed slow motion is 10.",
        get: |s| s.phase.tick_rate,
        set: |s, v| s.phase.tick_rate = v,
    },
    Spec {
        key: "phase.window_ticks",
        label: "Phase: allowed phase spread (ticks)",
        min: 1.0,
        max: 16.0,
        integer: false,
        explanation: "All labelled frames of one steady stretch must have their estimated phase inside an arc of \
this many ticks. One tick is unavoidable (the sprite changes in whole ticks while time is continuous); the rest is \
room for timestamp jitter from recording or re-encoding. Lower it to catch smaller mistakes at the price of more \
false alarms on jittery footage; raise it for very jittery footage.",
        get: |s| s.phase.window_ticks,
        set: |s, v| s.phase.window_ticks = v,
    },
    Spec {
        key: "phase.max_lag_ticks",
        label: "Phase: most ticks lag can explain",
        min: 0.0,
        max: 16.0,
        integer: false,
        explanation: "When the game stalls, its animation falls behind the video clock and then catches up, but \
Minecraft runs at most 10 catch-up ticks per frame, so a short lag excursion is never further off than about 10 \
ticks. A short odd stretch further than this from its neighbours cannot be lag and is marked as a hard error \
(for example a wrong sheet, which is exactly 16 ticks off); nearer ones are marked soft.",
        get: |s| s.phase.max_lag_ticks,
        set: |s, v| s.phase.max_lag_ticks = v,
    },
    Spec {
        key: "phase.min_shift_ticks",
        label: "Phase: smallest change worth flagging (ticks)",
        min: 0.0,
        max: 16.0,
        integer: false,
        explanation: "A change of the phase level smaller than this many ticks is treated as normal timing noise \
and not flagged. Real captures wobble: on the training recording 90% of frames sit within 0.7 tick of their local \
level and 1% stray beyond 2. One tick is the physical floor (the sprite changes in whole ticks). Set 0 to see \
every boundary, however small, at the price of many flags on jittery footage.",
        get: |s| s.phase.min_shift_ticks,
        set: |s, v| s.phase.min_shift_ticks = v,
    },
    Spec {
        key: "phase.max_excursion_seconds",
        label: "Phase: longest dip that counts as a lag stall (s)",
        min: 0.0,
        max: 3600.0,
        integer: false,
        explanation: "When the phase leaves its level and returns to the same level within this many seconds of \
video, it is reported once as an excursion (the game stalled for a moment and caught up), not as two separate \
shifts. A change that stays, or returns only later than this, is a shift: that is what a cut in the footage, a \
resource reload or leaving the world looks like. Set 0 to never merge.",
        get: |s| s.phase.max_excursion_seconds,
        set: |s, v| s.phase.max_excursion_seconds = v,
    },
    Spec {
        key: "index.irregular_deviation",
        label: "Timing: irregular interval (fraction of median)",
        min: 0.01,
        max: 10.0,
        integer: false,
        explanation: "The gap between two consecutive video frames is called irregular when it differs from the \
typical (median) gap by more than this fraction of that gap. 0.5 means a gap shorter than half or longer than \
one and a half times the usual one. Millisecond rounding in mkv files makes 16 ms and 17 ms gaps look alike at \
60 fps, which is why the default is generous.",
        get: |s| s.index.irregular_deviation,
        set: |s, v| s.index.irregular_deviation = v,
    },
    Spec {
        key: "index.vfr_fraction",
        label: "Timing: share of irregular gaps that means VFR",
        min: 0.0,
        max: 1.0,
        integer: false,
        explanation: "The video is reported as variable frame rate when more than this fraction of its frame gaps \
are irregular. The default 0.01 is 1%: a single dropped-footage hole in a long recording is listed as an irregular \
gap but does not make the whole video variable frame rate. Set 0 to call any irregular gap VFR.",
        get: |s| s.index.vfr_fraction,
        set: |s, v| s.index.vfr_fraction = v,
    },
    Spec {
        key: "project.fingerprint_window_bytes",
        label: "Project: bytes hashed at each end of the video",
        min: 4096.0,
        max: 1_073_741_824.0,
        integer: true,
        explanation: "To notice that a project is opened with a different video than it was made from, the video is \
fingerprinted by its size plus a hash of its first and last N bytes. A larger window is more reliable but reads \
more (2 x N bytes per open); bytes in the middle of a very large file are never hashed. The window used is \
recorded inside each project, so changing this does not break existing projects.",
        get: |s| s.project.fingerprint_window_bytes as f64,
        set: |s, v| s.project.fingerprint_window_bytes = v as u64,
    },
    Spec {
        key: "label.advance_frames",
        label: "Label: frames to advance after labelling",
        min: 0.0,
        max: 10000.0,
        integer: true,
        explanation: "After a label is set, the playhead moves this many frames forward, so labelling a fire \
frame by frame is one click per frame. 1 is frame by frame; 3 matches one game tick at 60 fps (the sprite only \
changes every tick, so most frames repeat the previous row); 0 stays on the frame, for correcting it.",
        get: |s| s.label.advance_frames as f64,
        set: |s, v| s.label.advance_frames = v as u32,
    },
    Spec {
        key: "label.big_step_frames",
        label: "Label: frames moved by the big-step keys",
        min: 2.0,
        max: 100000.0,
        integer: true,
        explanation: "How many frames Shift plus the arrow keys move the playhead. Ten frames is a sixth of a \
second at 60 fps; raise it to cross long stretches faster, lower it for finer scrubbing.",
        get: |s| s.label.big_step_frames as f64,
        set: |s, v| s.label.big_step_frames = v as u32,
    },
    Spec {
        key: "label.advance_to_unlabelled",
        label: "Label: jump to the next unlabelled frame",
        min: 0.0,
        max: 1.0,
        integer: true,
        explanation: "1 = after labelling, skip forward to the next frame that has no label yet instead of \
moving by the fixed number of frames (useful when revisiting a half-labelled video); 0 = always move by the \
configured number of frames. If there is no unlabelled frame ahead the playhead stays.",
        get: |s| if s.label.advance_to_unlabelled { 1.0 } else { 0.0 },
        set: |s, v| s.label.advance_to_unlabelled = v != 0.0,
    },
    Spec {
        key: "label.row_type_timeout_ms",
        label: "Label: wait before committing a single typed digit (ms)",
        min: 100.0,
        max: 5000.0,
        integer: true,
        explanation: "Rows are typed as digits. Typing 4 to 9 labels at once, but 0 to 3 could be the start of a \
two-digit row (12, 31...), so the program waits this long for a second digit and then commits the single one. \
Lower it if you only ever type single digits; raise it if you type slowly. Enter commits immediately.",
        get: |s| s.label.row_type_timeout_ms as f64,
        set: |s, v| s.label.row_type_timeout_ms = v as u32,
    },
    Spec {
        key: "label.suggest_max_seconds",
        label: "Label: furthest a row suggestion reaches (s)",
        min: 0.0,
        max: 3600.0,
        integer: false,
        explanation: "The suggested row for a frame is computed from the nearest labelled fire frame by counting \
game ticks of video time. The longer the distance, the more a timing error or a cut can throw it off, so no \
suggestion is made further than this many seconds from a label. Set 0 to turn suggestions off.",
        get: |s| s.label.suggest_max_seconds,
        set: |s, v| s.label.suggest_max_seconds = v,
    },
    Spec {
        key: "label.max_play_speed",
        label: "Label: fastest playback (x frame rate)",
        min: 1.0,
        max: 64.0,
        integer: true,
        explanation: "L plays forward and J backward; pressing the same key again doubles the speed up to this \
multiple of the video's frame rate (1, 2, 4, 8...). K or Space stops. Very high speeds just skip frames, so \
raise it only to scan long footage quickly.",
        get: |s| s.label.max_play_speed as f64,
        set: |s, v| s.label.max_play_speed = v as u32,
    },
    Spec {
        key: "label.min_play_tick_ms",
        label: "Label: shortest playback step (ms)",
        min: 4.0,
        max: 200.0,
        integer: true,
        explanation: "While playing, the picture advances at most once per this many milliseconds. Playing faster \
than that (for example 8x of a 60 fps video) moves several frames per step instead, so the speed is kept without \
overloading the screen and the decoder. 16 ms is one refresh of a 60 Hz monitor.",
        get: |s| s.label.min_play_tick_ms as f64,
        set: |s, v| s.label.min_play_tick_ms = v as u32,
    },
    Spec {
        key: "decode.window_back",
        label: "Decode: frames kept ready behind the playhead",
        min: 0.0,
        max: 100000.0,
        integer: true,
        explanation: "Decoded frames are kept in memory around the playhead so stepping is instant. This many \
frames behind the current one are kept ready. Stepping backwards past this window needs a new decode, which \
starts at the previous keyframe.",
        get: |s| s.decode.window_back as f64,
        set: |s, v| s.decode.window_back = v as u32,
    },
    Spec {
        key: "decode.window_ahead",
        label: "Decode: frames kept ready ahead of the playhead",
        min: 1.0,
        max: 100000.0,
        integer: true,
        explanation: "This many frames ahead of the current one are decoded in advance, so playing and stepping \
forward never waits. Larger windows mean fewer decoder restarts but more memory.",
        get: |s| s.decode.window_ahead as f64,
        set: |s, v| s.decode.window_ahead = v as u32,
    },
    Spec {
        key: "decode.prefetch_fraction",
        label: "Decode: refill when the ready frames drop below",
        min: 0.0,
        max: 1.0,
        integer: false,
        explanation: "More frames are decoded only when the ready frames on one side fall below this fraction of \
that side's window (then a whole window is fetched at once). A high value keeps the window topped up but starts \
the decoder often; a low value fetches rarely. 0 never prefetches ahead of need.",
        get: |s| s.decode.prefetch_fraction,
        set: |s, v| s.decode.prefetch_fraction = v,
    },
    Spec {
        key: "decode.cache_mb",
        label: "Decode: frame cache size (MiB)",
        min: 16.0,
        max: 65536.0,
        integer: true,
        explanation: "Memory used for decoded frames. The least recently viewed frames are dropped when it is \
full. A frame bigger than the whole cache is still kept while it is shown.",
        get: |s| s.decode.cache_mb as f64,
        set: |s, v| s.decode.cache_mb = v as u32,
    },
    Spec {
        key: "decode.max_width",
        label: "Decode: widest decoded frame (px)",
        min: 160.0,
        max: 7680.0,
        integer: true,
        explanation: "Frames are scaled down to at most this width before they are shown. Fire sprites are small, \
so raise it for 4K footage when you need to see detail; lower it to decode faster and use less memory.",
        get: |s| s.decode.max_width as f64,
        set: |s, v| s.decode.max_width = v as u32,
    },
    Spec {
        key: "decode.max_gop_frames",
        label: "Decode: slow-seek warning distance (frames)",
        min: 1.0,
        max: 1000000.0,
        integer: true,
        explanation: "Video can only be decoded from a keyframe. When the keyframe before the wanted frame is more \
than this many frames away (long keyframe intervals, typical of streaming downloads) the jump is marked as slow \
so the interface can show a spinner instead of looking frozen.",
        get: |s| s.decode.max_gop_frames as f64,
        set: |s, v| s.decode.max_gop_frames = v as u32,
    },
    Spec {
        key: "decode.seek_backoff_ms",
        label: "Decode: seek this far before the keyframe (ms)",
        min: 0.0,
        max: 100.0,
        integer: false,
        explanation: "To show a frame, the video is decoded from the keyframe before it. Timestamps are stored \
rounded, so a seek aimed exactly at the keyframe can land a hair after it and skip it, which would shift every \
frame of that decode by one. Aiming this many milliseconds early avoids that; it must stay smaller than the gap \
between two frames.",
        get: |s| s.decode.seek_backoff_ms,
        set: |s, v| s.decode.seek_backoff_ms = v,
    },
    Spec {
        key: "decode.align_tolerance",
        label: "Decode: allowed timestamp mismatch (frame intervals)",
        min: 0.01,
        max: 10.0,
        integer: false,
        explanation: "Every decoded frame is checked against the frame index: its timestamp must match the \
index entry it will be shown as, within this fraction of the local frame interval. A mismatch means the picture \
would not be the frame you think (a dropped frame, an index from another file) and is reported as an error \
instead of shown. 0.5 is half a frame; raise it only to accept sloppy timestamps knowingly.",
        get: |s| s.decode.align_tolerance,
        set: |s, v| s.decode.align_tolerance = v,
    },
    Spec {
        key: "decode.jpeg_quality",
        label: "Decode: JPEG quality (2 best, 31 worst)",
        min: 2.0,
        max: 31.0,
        integer: true,
        explanation: "Decoded frames travel to the screen as JPEG. This is ffmpeg's quantiser scale: 2 is the \
best quality and the largest pictures, 31 the smallest and blockiest. Fire sprites are small, so keep it low \
(good quality) unless transfers are slow.",
        get: |s| s.decode.jpeg_quality as f64,
        set: |s, v| s.decode.jpeg_quality = v as u32,
    },
    Spec {
        key: "autosave.compact_after_lines",
        label: "Autosave: rewrite the label log after (lines)",
        min: 1000.0,
        max: 10000000.0,
        integer: true,
        explanation: "Every label change is appended to a log, so nothing is lost on a crash, and the log grows \
with every correction. When it holds more than this many lines it is rewritten with just the current labels. \
Raise it to rewrite less often.",
        get: |s| s.autosave.compact_after_lines as f64,
        set: |s, v| s.autosave.compact_after_lines = v as u32,
    },
    Spec {
        key: "autosave.fsync",
        label: "Autosave: flush every change to disk",
        min: 0.0,
        max: 1.0,
        integer: true,
        explanation: "1 = force every label change onto the disk before carrying on, so a power cut or crash \
loses nothing; 0 = let the operating system write when it likes, which is slightly faster but can lose the last \
few labels on a crash.",
        get: |s| if s.autosave.fsync { 1.0 } else { 0.0 },
        set: |s, v| s.autosave.fsync = v != 0.0,
    },
    Spec {
        key: "timeline.zoom_factor",
        label: "Timeline: zoom per wheel notch",
        min: 1.05,
        max: 4.0,
        integer: false,
        explanation: "How much the timeline zooms for each notch of the mouse wheel. 1.25 means 25% more or less. \
The frame under the mouse pointer stays under it while zooming.",
        get: |s| s.timeline.zoom_factor,
        set: |s, v| s.timeline.zoom_factor = v,
    },
    Spec {
        key: "timeline.max_px_per_frame",
        label: "Timeline: most zoomed in (pixels per frame)",
        min: 1.0,
        max: 256.0,
        integer: false,
        explanation: "The furthest the timeline zooms in, as screen pixels per video frame. At 32 a single frame \
is a wide, easy target for clicking; lower values stop zooming earlier.",
        get: |s| s.timeline.max_px_per_frame,
        set: |s, v| s.timeline.max_px_per_frame = v,
    },
    Spec {
        key: "timeline.thumb_width_px",
        label: "Timeline: thumbnail width (px)",
        min: 32.0,
        max: 512.0,
        integer: true,
        explanation: "Width of the small preview pictures along the timeline. Wider thumbnails show more detail \
but fewer of them fit, and each one has to be decoded.",
        get: |s| s.timeline.thumb_width_px as f64,
        set: |s, v| s.timeline.thumb_width_px = v as u32,
    },
    Spec {
        key: "timeline.thumb_quality",
        label: "Timeline: thumbnail JPEG quality (2 best - 31 smallest)",
        min: 2.0,
        max: 31.0,
        integer: true,
        explanation: "How strongly the timeline thumbnails are compressed (ffmpeg's MJPEG quality scale: 2 is the \
best picture and the largest file, 31 the smallest and blockiest). Thumbnails are tiny, so the default of 5 costs \
only a few kilobytes each.",
        get: |s| s.timeline.thumb_quality as f64,
        set: |s, v| s.timeline.thumb_quality = v as u32,
    },
    Spec {
        key: "timeline.thumb_max_count",
        label: "Timeline: most thumbnails per video",
        min: 1.0,
        max: 100_000.0,
        integer: true,
        explanation: "Thumbnails are taken from the video's keyframes in one fast pass. If the video has more \
keyframes than this, they are thinned evenly (every 2nd, 3rd, ...) so that no more than this many are kept. A lower \
number is quicker to generate and uses less disk; a higher number gives a denser strip when zoomed in.",
        get: |s| s.timeline.thumb_max_count as f64,
        set: |s, v| s.timeline.thumb_max_count = v as u32,
    },
    Spec {
        key: "timeline.thumbs_enabled",
        label: "Timeline: show thumbnails",
        min: 0.0,
        max: 1.0,
        integer: true,
        explanation: "1 = generate the thumbnail strip in the background when a video is opened (it is cached in \
the project folder, so this happens once); 0 = never generate or draw it, e.g. on a slow disk or for a huge video.",
        get: |s| if s.timeline.thumbs_enabled { 1.0 } else { 0.0 },
        set: |s, v| s.timeline.thumbs_enabled = v != 0.0,
    },
    Spec {
        key: "timeline.follow_margin_px",
        label: "Timeline: playhead margin while following (px)",
        min: 0.0,
        max: 2000.0,
        integer: true,
        explanation: "While the video plays or you step, the timeline scrolls just enough to keep the playhead at \
least this many pixels from its left and right edges. A margin of half the timeline width or more keeps the \
playhead centred; 0 scrolls only when it would leave the view.",
        get: |s| s.timeline.follow_margin_px as f64,
        set: |s, v| s.timeline.follow_margin_px = v as u32,
    },
    Spec {
        key: "timeline.ruler_spacing_px",
        label: "Timeline: closest ruler labels (px)",
        min: 10.0,
        max: 1000.0,
        integer: true,
        explanation: "The time ruler picks the smallest round time step (1 s, 5 s, 1 min...) whose labels are at \
least this many pixels apart. Lower values give a denser ruler; raise it if the labels crowd each other.",
        get: |s| s.timeline.ruler_spacing_px as f64,
        set: |s, v| s.timeline.ruler_spacing_px = v as u32,
    },
    Spec {
        key: "timeline.drag_threshold_px",
        label: "Timeline: click or drag (px)",
        min: 1.0,
        max: 100.0,
        integer: true,
        explanation: "A mouse press on the timeline that moves less than this many pixels before release is a \
click and jumps to that frame; moving further is a drag and pans the timeline instead. Raise it if clicks are \
taken for drags (shaky hand, touch pad).",
        get: |s| s.timeline.drag_threshold_px as f64,
        set: |s, v| s.timeline.drag_threshold_px = v as u32,
    },
];

fn spec(key: &str) -> Result<&'static Spec, SettingError> {
    SPECS.iter().find(|sp| sp.key == key).ok_or_else(|| SettingError::UnknownKey(key.to_string()))
}

impl Settings {
    /// Every setting, in display order.
    pub fn describe(&self) -> Vec<SettingInfo> {
        let defaults = Settings::default();
        SPECS
            .iter()
            .map(|sp| SettingInfo {
                key: sp.key,
                label: sp.label,
                value: (sp.get)(self),
                default: (sp.get)(&defaults),
                min: sp.min,
                max: sp.max,
                integer: sp.integer,
                explanation: sp.explanation,
            })
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        spec(key).ok().map(|sp| (sp.get)(self))
    }

    /// Validate and set one setting; on error nothing changes.
    pub fn set(&mut self, key: &str, value: f64) -> Result<(), SettingError> {
        let sp = spec(key)?;
        if !value.is_finite() {
            return Err(SettingError::NotFinite(key.to_string()));
        }
        if value < sp.min || value > sp.max {
            return Err(SettingError::OutOfRange { key: key.to_string(), value, min: sp.min, max: sp.max });
        }
        if sp.integer && value.fract() != 0.0 {
            return Err(SettingError::NotAnInteger(key.to_string()));
        }
        (sp.set)(self, value);
        Ok(())
    }

    /// Put one setting back to its default.
    pub fn reset(&mut self, key: &str) -> Result<(), SettingError> {
        let sp = spec(key)?;
        let default = (sp.get)(&Settings::default());
        (sp.set)(self, default);
        Ok(())
    }
}
