import os
import shutil
import subprocess

import pytest

import calibrate
from calibrate import CalibrateError, analyze, nearest_rung

pytestmark = pytest.mark.skipif(
    shutil.which("ffmpeg") is None or shutil.which("ffprobe") is None, reason="ffmpeg/ffprobe not installed"
)


def encode(path, args, seconds=4):
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=30",
         "-t", str(seconds), *args, str(path)],
        check=True,
    )
    return path


@pytest.fixture(scope="module")
def clips(tmp_path_factory):
    d = tmp_path_factory.mktemp("cal")
    return {
        "bframes": encode(d / "b.mp4", ["-c:v", "libx264", "-g", "30", "-bf", "2", "-profile:v", "high",
                                        "-b:v", "2M", "-pix_fmt", "yuv420p", "-sc_threshold", "0"]),
        "nob": encode(d / "nob.mp4", ["-c:v", "libx264", "-g", "60", "-bf", "0", "-profile:v", "high",
                                      "-b:v", "2M", "-pix_fmt", "yuv420p", "-sc_threshold", "0"]),
        "vp9": encode(d / "v.webm", ["-c:v", "libvpx-vp9", "-g", "60", "-b:v", "1M", "-pix_fmt", "yuv420p"]),
    }


# ---------- what it recovers ----------

def test_recovers_codec_profile_geometry_and_format(clips):
    s = analyze(clips["bframes"])
    assert s["codec"] == "h264" and s["profile"] == "High"
    assert (s["width"], s["height"]) == (1280, 720)
    assert abs(s["fps"] - 30.0) < 0.01
    assert s["pix_fmt"] == "yuv420p"
    assert s["frames"] == 120 and abs(s["duration_s"] - 4.0) < 0.1


def test_recovers_the_keyframe_interval(clips):
    assert analyze(clips["bframes"])["keyframe_interval"]["median"] == 30
    assert analyze(clips["nob"])["keyframe_interval"]["median"] == 60


def test_tells_b_frames_from_none(clips):
    assert analyze(clips["bframes"])["b_frame_ratio"] > 0.2
    assert analyze(clips["nob"])["b_frame_ratio"] == 0.0


def test_bitrate_follows_the_file_size(clips):
    s = analyze(clips["bframes"])
    expected = os.path.getsize(clips["bframes"]) * 8 / s["duration_s"] / 1000
    assert abs(s["bitrate_kbps"] - expected) / expected < 0.15


def test_other_codecs(clips):
    s = analyze(clips["vp9"])
    assert s["codec"] == "vp9"
    assert s["keyframe_interval"]["median"] == 60


# ---------- matching a ladder rung ----------

def test_nearest_rung_snaps_the_height_and_names_the_codec():
    assert nearest_rung({"codec": "h264", "height": 720}) == {"codec": "h264", "height": 720}
    assert nearest_rung({"codec": "vp9", "height": 1000})["height"] == 1080
    assert nearest_rung({"codec": "av1", "height": 100})["height"] == 144
    assert nearest_rung({"codec": "h264", "height": 4000})["height"] == 2160


# ---------- failures ----------

def test_missing_and_non_video_files(tmp_path):
    with pytest.raises(FileNotFoundError):
        analyze(tmp_path / "nope.mp4")
    bad = tmp_path / "bad.mp4"
    bad.write_text("not a video")
    with pytest.raises(CalibrateError):
        analyze(bad)


def test_the_command_line_prints_a_row_per_file(clips, capsys):
    assert calibrate.main([str(clips["bframes"]), str(clips["nob"])]) == 0
    out = capsys.readouterr().out
    assert "b.mp4" in out and "nob.mp4" in out and "h264" in out


def test_the_command_line_reports_a_bad_file_and_exits_non_zero(tmp_path, capsys):
    bad = tmp_path / "bad.mp4"
    bad.write_text("nope")
    assert calibrate.main([str(bad)]) == 1
    assert "error" in capsys.readouterr().err.lower()
