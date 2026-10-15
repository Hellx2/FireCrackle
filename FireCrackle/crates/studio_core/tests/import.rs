use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use studio_core::config::Settings;
use studio_core::import::*;
use studio_core::project::{fingerprint, Project, ProjectError, VideoMeta};

// ------------------------------------------------------------ helpers

fn have_tools() -> bool {
    ["ffmpeg", "ffprobe"].iter().all(|t| {
        Command::new(t).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    })
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fc_import_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Encode a plain gray test video of `n` frames (brightness changes per frame so the encoder keeps them).
/// x264, mkv. Nothing is stamped into the picture: footage has no sync strip.
fn make_video(path: &Path, w: usize, h: usize, fps: u32, n: usize, extra: &[&str]) {
    let mut child = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "gray", "-s", &format!("{w}x{h}")])
        .args(["-r", &fps.to_string(), "-i", "-", "-c:v", "libx264", "-preset", "veryfast", "-crf", "14"])
        .args(["-pix_fmt", "yuv420p"])
        .args(extra)
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for i in 0..n {
            stdin.write_all(&vec![(40 + (i * 5) % 150) as u8; w * h]).unwrap();
        }
    }
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success(), "ffmpeg failed to encode the test video");
}

fn import(video: &Path) -> Result<Project, ImportError> {
    import_video(&Tools::default(), video, &Settings::default())
}

// ------------------------------------------------- parse_stream_info

#[test]
fn stream_info_parses_the_real_ffprobe_output() {
    let text = "codec_name=h264\nwidth=1920\nheight=1080\nr_frame_rate=60/1\n";
    assert_eq!(
        parse_stream_info(text).unwrap(),
        VideoMeta { width: 1920, height: 1080, fps_num: 60, fps_den: 1, codec: "h264".into() }
    );
}

#[test]
fn stream_info_handles_ntsc_rates_crlf_and_key_order() {
    let text = "r_frame_rate=30000/1001\r\nheight=720\r\nwidth=1280\r\ncodec_name=vp9\r\n";
    let m = parse_stream_info(text).unwrap();
    assert_eq!((m.width, m.height, m.fps_num, m.fps_den), (1280, 720, 30000, 1001));
    assert_eq!(m.codec, "vp9");
}

#[test]
fn stream_info_takes_the_first_value_of_a_repeated_key() {
    let m = parse_stream_info("width=640\nheight=360\nr_frame_rate=25/1\nwidth=1\nheight=1\n").unwrap();
    assert_eq!((m.width, m.height), (640, 360));
}

#[test]
fn stream_info_without_a_codec_line_says_unknown() {
    let m = parse_stream_info("width=640\nheight=360\nr_frame_rate=25/1\n").unwrap();
    assert_eq!(m.codec, "unknown");
}

#[test]
fn stream_info_empty_output_means_no_video_stream() {
    assert_eq!(parse_stream_info(""), Err(ImportError::NoVideoStream));
    assert_eq!(parse_stream_info("  \r\n\n"), Err(ImportError::NoVideoStream));
}

#[test]
fn stream_info_garbage_is_a_probe_failure_not_a_panic() {
    for text in [
        "width=abc\nheight=10\nr_frame_rate=25/1\n",
        "width=10\nheight=10\nr_frame_rate=0/0\n",
        "width=10\nheight=10\nr_frame_rate=25\n",
        "width=10\nheight=10\n",
        "width=0\nheight=10\nr_frame_rate=25/1\n",
        "width=10\nheight=-4\nr_frame_rate=25/1\n",
        "complete nonsense\n",
    ] {
        assert!(matches!(parse_stream_info(text), Err(ImportError::ProbeFailed(_))), "{text:?}");
    }
}

#[test]
fn default_tools_are_looked_up_on_path() {
    let t = Tools::default();
    assert_eq!(t.ffmpeg, "ffmpeg");
    assert_eq!(t.ffprobe, "ffprobe");
}

// ------------------------------------------------ errors (no tools)

#[test]
fn missing_tools_are_a_typed_error() {
    let tools = Tools { ffmpeg: "definitely-not-ffmpeg-xyz".into(), ffprobe: "definitely-not-ffprobe-xyz".into() };
    let r = import_video(&tools, Path::new("whatever.mkv"), &Settings::default());
    assert!(matches!(r, Err(ImportError::ToolMissing(_))), "{r:?}");
    let r = probe(&tools, Path::new("whatever.mkv"), &Settings::default().index);
    assert!(matches!(r, Err(ImportError::ToolMissing(_))), "{r:?}");
}

// --------------------------------------------- errors (real ffprobe)

#[test]
fn a_missing_file_and_a_non_video_are_probe_failures() {
    if !have_tools() {
        eprintln!("ffmpeg not found, skipping");
        return;
    }
    let dir = scratch("errors");
    assert!(matches!(import(&dir.join("nope.mkv")), Err(ImportError::ProbeFailed(_))));
    let txt = dir.join("notes.mkv");
    std::fs::write(&txt, "this is not a video at all").unwrap();
    assert!(matches!(import(&txt), Err(ImportError::ProbeFailed(_)) | Err(ImportError::NoVideoStream)));
    let empty = dir.join("empty.mkv");
    std::fs::write(&empty, b"").unwrap();
    assert!(import(&empty).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------- full import

#[test]
fn a_plain_video_imports_into_a_display_order_index() {
    if !have_tools() {
        eprintln!("ffmpeg not found, skipping");
        return;
    }
    let dir = scratch("clean");
    let v = dir.join("clip.mkv");
    make_video(&v, 1280, 720, 30, 24, &[]);
    let p = import(&v).unwrap();

    assert_eq!(p.frames.len(), 24);
    assert!(p.frames.windows(2).all(|w| w[0].pts < w[1].pts), "display order");
    assert!(p.frames[0].key);
    assert_eq!((p.meta.width, p.meta.height, p.meta.fps_num, p.meta.fps_den), (1280, 720, 30, 1));
    assert_eq!(p.meta.codec, "h264");
    assert!((p.index.fps - 30.0).abs() < 0.5, "fps {}", p.index.fps);
    assert!(!p.index.vfr);
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    assert_eq!(p.video_size, std::fs::metadata(&v).unwrap().len());
    assert_eq!(p.video_path, v.to_string_lossy());
    assert_eq!(p.verify_video(), Ok(()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn any_width_imports_there_is_no_minimum_for_a_strip() {
    if !have_tools() {
        return;
    }
    let dir = scratch("narrow");
    let v = dir.join("small.mkv");
    make_video(&v, 320, 180, 30, 8, &[]);
    let p = import(&v).unwrap();
    assert_eq!(p.frames.len(), 8);
    assert_eq!(p.meta.width, 320);
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn b_frame_video_is_indexed_in_display_order_with_keyframes_in_place() {
    if !have_tools() {
        return;
    }
    let dir = scratch("bframes");
    let v = dir.join("b.mkv");
    make_video(&v, 1280, 720, 30, 40, &["-bf", "3", "-g", "8", "-keyint_min", "8", "-sc_threshold", "0"]);

    // the test only means something if the packets really are reordered
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "packet=pts_time", "-of", "csv=p=0"])
        .arg(&v)
        .output()
        .unwrap();
    let pts: Vec<f64> = String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.trim().parse().ok()).collect();
    assert!(pts.windows(2).any(|w| w[1] < w[0]), "no B-frame reordering in the test video");

    let p = import(&v).unwrap();
    assert_eq!(p.frames.len(), 40);
    assert!(p.frames.windows(2).all(|w| w[0].pts < w[1].pts));
    for n in [0usize, 8, 16, 24, 32] {
        assert!(p.frames[n].key, "keyframe flag must follow display order: frame {n}");
    }
    assert!(!p.frames[5].key);
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn paths_with_spaces_and_non_ascii_import() {
    if !have_tools() {
        return;
    }
    let dir = scratch("paths").join("my clips ü 火");
    std::fs::create_dir_all(&dir).unwrap();
    let v = dir.join("2026-10-15 14-15-51 ü.mkv");
    make_video(&v, 1280, 720, 30, 9, &[]);
    let p = import(&v).unwrap();
    assert_eq!(p.frames.len(), 9);
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn the_fingerprint_window_setting_is_used() {
    if !have_tools() {
        return;
    }
    let dir = scratch("fpwin");
    let v = dir.join("f.mkv");
    make_video(&v, 1280, 720, 30, 24, &[]);
    let mut s = Settings::default();
    s.set("project.fingerprint_window_bytes", 4096.0).unwrap();
    let p = import_video(&Tools::default(), &v, &s).unwrap();
    assert_eq!(p.fingerprint, fingerprint(&v, 4096).unwrap());
    assert!(p.fingerprint.contains(":4096:"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_index_irregularity_setting_is_used() {
    if !have_tools() {
        return;
    }
    let dir = scratch("irreg");
    let v = dir.join("i.mkv");
    make_video(&v, 640, 360, 30, 90, &[]);
    // ms-rounded 33/34 ms intervals: regular with the default, "irregular" with an absurdly tight limit
    assert!(!import(&v).unwrap().index.vfr);
    let mut s = Settings::default();
    s.set("index.irregular_deviation", 0.01).unwrap();
    let p = import_video(&Tools::default(), &v, &s).unwrap();
    assert!(p.index.vfr, "{:?}", p.index);
    assert!(p.warnings.iter().any(|w| w.to_lowercase().contains("variable frame rate")), "{:?}", p.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_imported_project_survives_save_load_and_detects_a_changed_video() {
    if !have_tools() {
        return;
    }
    let dir = scratch("saveload");
    let v = dir.join("s.mkv");
    make_video(&v, 1280, 720, 30, 24, &[]);
    let p = import(&v).unwrap();
    let file = dir.join("s.firecrackle.json");
    p.save(&file).unwrap();
    let back = Project::load(&file).unwrap();
    assert_eq!(back, p);
    assert_eq!(back.verify_video(), Ok(()));

    // someone re-records over it
    let mut bytes = std::fs::read(&v).unwrap();
    bytes[10] ^= 0xFF;
    std::fs::write(&v, &bytes).unwrap();
    assert_eq!(back.verify_video(), Err(ProjectError::VideoChanged));
    let _ = std::fs::remove_dir_all(&dir);
}
