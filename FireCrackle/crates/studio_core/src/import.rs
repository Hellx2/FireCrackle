//! Video import: ffprobe (stream + packets) -> display-order index -> `Project`. No frame is decoded here.

use crate::config::{IndexConfig, Settings};
use crate::index::{build_index, parse_packets, FrameIndex, IndexError};
use crate::project::{fingerprint, Project, VideoMeta};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

/// Ubuntu 22.04's FFmpeg 4.4 predates `-fps_mode`; both spellings preserve every frame.
pub const PASSTHROUGH_ARGS: [&str; 2] = if cfg!(target_os = "linux") {
    ["-vsync", "0"]
} else {
    ["-fps_mode", "passthrough"]
};

/// Preserve selected timestamps without duplicating frames after a selection filter.
pub const VARIABLE_FRAME_RATE_ARGS: [&str; 2] = if cfg!(target_os = "linux") {
    ["-vsync", "2"]
} else {
    ["-fps_mode", "vfr"]
};

/// Video tools shipped next to the app, with PATH fallback for development.
#[derive(Clone, Debug)]
pub struct Tools {
    pub ffmpeg: OsString,
    pub ffprobe: OsString,
}

impl Default for Tools {
    fn default() -> Tools {
        Self::beside(
            std::env::current_exe()
                .ok()
                .as_deref()
                .and_then(Path::parent),
        )
    }
}

impl Tools {
    /// Resolve tools independently, using absolute paths so shortcuts and other working directories work.
    pub fn beside(directory: Option<&Path>) -> Tools {
        let resolve = |name: &str| {
            let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            directory
                .map(|dir| dir.join("tools").join(filename))
                .filter(|path| path.is_file())
                .map(|path| path.into_os_string())
                .unwrap_or_else(|| name.into())
        };
        Tools {
            ffmpeg: resolve("ffmpeg"),
            ffprobe: resolve("ffprobe"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    /// ffmpeg/ffprobe could not be started (missing or not executable).
    ToolMissing(String),
    /// ffprobe ran but could not read the file (not a video, unreadable, ...), or printed unusable output.
    ProbeFailed(String),
    /// ffprobe found no video stream.
    NoVideoStream,
    BadPackets(IndexError),
    Io(String),
}

/// Parse `ffprobe -show_entries stream=codec_name,width,height,r_frame_rate -of default=nw=1` output
/// (`key=value` lines in any order, CRLF tolerated). Empty output means there is no video stream.
pub fn parse_stream_info(text: &str) -> Result<VideoMeta, ImportError> {
    if text.trim().is_empty() {
        return Err(ImportError::NoVideoStream);
    }
    // the first value of each key wins (several streams may be printed)
    let first = |key: &str| {
        text.lines()
            .filter_map(|l| l.trim().split_once('='))
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.trim())
    };
    let dim = |key: &str| -> Result<usize, ImportError> {
        let v = first(key)
            .ok_or_else(|| ImportError::ProbeFailed(format!("ffprobe output has no {key}")))?;
        match v.parse::<usize>() {
            Ok(n) if n > 0 => Ok(n),
            _ => Err(ImportError::ProbeFailed(format!(
                "bad {key} in ffprobe output: {v:?}"
            ))),
        }
    };
    let (width, height) = (dim("width")?, dim("height")?);
    let rate = first("r_frame_rate")
        .ok_or_else(|| ImportError::ProbeFailed("ffprobe output has no frame rate".to_string()))?;
    let bad_rate =
        || ImportError::ProbeFailed(format!("unusable frame rate in ffprobe output: {rate:?}"));
    let (n, d) = rate.split_once('/').ok_or_else(bad_rate)?;
    let (fps_num, fps_den): (u32, u32) = (
        n.parse().map_err(|_| bad_rate())?,
        d.parse().map_err(|_| bad_rate())?,
    );
    if fps_num == 0 || fps_den == 0 {
        return Err(bad_rate());
    }
    let codec = first("codec_name")
        .filter(|c| !c.is_empty())
        .unwrap_or("unknown")
        .to_string();
    Ok(VideoMeta {
        width,
        height,
        fps_num,
        fps_den,
        codec,
    })
}

/// Run an ffprobe query, returning stdout. A tool that cannot start is `ToolMissing`; a non-zero exit is
/// `ProbeFailed` with ffprobe's own message.
fn ffprobe(tools: &Tools, args: &[&str], video: &Path) -> Result<String, ImportError> {
    let out = Command::new(&tools.ffprobe)
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(args)
        .arg(video)
        .output()
        .map_err(|e| ImportError::ToolMissing(format!("could not run {:?}: {e}", tools.ffprobe)))?;
    if !out.status.success() {
        return Err(ImportError::ProbeFailed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Probe stream info and the frame index (no decoding).
pub fn probe(
    tools: &Tools,
    video: &Path,
    cfg: &IndexConfig,
) -> Result<(VideoMeta, FrameIndex), ImportError> {
    let info = ffprobe(
        tools,
        &[
            "-show_entries",
            "stream=codec_name,width,height,r_frame_rate",
            "-of",
            "default=noprint_wrappers=1",
        ],
        video,
    )?;
    let meta = parse_stream_info(&info)?;
    let packets_text = ffprobe(
        tools,
        &[
            "-show_entries",
            "packet=pts_time,dts_time,flags",
            "-of",
            "csv=p=0",
        ],
        video,
    )?;
    let packets = parse_packets(&packets_text).map_err(ImportError::BadPackets)?;
    let index = build_index(&packets, cfg).map_err(ImportError::BadPackets)?;
    Ok((meta, index))
}

/// Full import: probe, fingerprint, assemble the project.
pub fn import_video(
    tools: &Tools,
    video: &Path,
    settings: &Settings,
) -> Result<Project, ImportError> {
    let (meta, index) = probe(tools, video, &settings.index)?;
    let size = std::fs::metadata(video)
        .map_err(|e| ImportError::Io(e.to_string()))?
        .len();
    let fp = fingerprint(video, settings.project.fingerprint_window_bytes)
        .map_err(|e| ImportError::Io(e.to_string()))?;
    Ok(Project::assemble(
        &video.to_string_lossy(),
        size,
        fp,
        meta,
        &index,
    ))
}
