import json
import random
import shutil

import pytest

import robustness
from profiles import Source, sample_chain
from robustness import decode_accuracy, find_binaries, run_matrix

try:
    DECODE, GEN = find_binaries()
except FileNotFoundError:
    DECODE = GEN = None

pytestmark = pytest.mark.skipif(
    DECODE is None or shutil.which("ffmpeg") is None, reason="needs ffmpeg and `cargo build --release` in core/"
)

FRAMES = 24
SIZE = (1280, 720)


@pytest.fixture(scope="module")
def source(tmp_path_factory):
    return robustness.build_source(GEN, tmp_path_factory.mktemp("src") / "src.mp4", FRAMES, SIZE, 30)


def chain(codec, height, recorder=False, seed=1):
    return sample_chain(random.Random(seed), Source(SIZE[0], SIZE[1], 30.0), codec=codec, height=height,
                        with_recorder=recorder, seed=seed)


def test_a_clean_source_decodes_perfectly(source):
    r = decode_accuracy(DECODE, source, FRAMES)
    assert r == {"status": "ok", "frames": FRAMES, "decoded": FRAMES, "correct": FRAMES}


def test_h264_at_the_same_resolution_keeps_the_strip_readable(source, tmp_path):
    rows = run_matrix(source, tmp_path, [chain("h264", 720, recorder=True)], DECODE, FRAMES)
    row = rows[0]
    assert row["status"] == "ok" and row["codec"] == "h264" and row["height"] == 720
    assert row["frames"] == FRAMES, "the chain must keep every frame"
    assert row["correct"] / row["frames"] >= 0.9, row


def test_a_downscaled_vp9_rung_is_measured(source, tmp_path):
    row = run_matrix(source, tmp_path, [chain("vp9", 480)], DECODE, FRAMES)[0]
    assert row["strip_decodable"] is True and row["width"] == 854
    assert row["status"] == "ok" and row["decoded"] > 0


def test_a_rung_under_480_px_wide_is_reported_not_guessed(source, tmp_path):
    row = run_matrix(source, tmp_path, [chain("h264", 240)], DECODE, FRAMES)[0]
    assert row["strip_decodable"] is False and row["width"] == 426
    assert row["status"] == "too_narrow"
    assert row["decoded"] == 0 and row["correct"] == 0


def test_an_encoder_failure_becomes_an_error_row_not_a_crash(tmp_path):
    missing = tmp_path / "missing.mp4"
    rows = run_matrix(missing, tmp_path, [chain("h264", 720)], DECODE, FRAMES)
    assert rows[0]["status"] == "error" and rows[0]["message"]


def test_the_command_line_writes_a_json_report(tmp_path):
    rc = robustness.main(["--frames", "12", "--size", "1280x720", "--codecs", "h264", "--heights", "720",
                          "--seeds", "1", "--out", str(tmp_path)])
    assert rc == 0
    report = json.loads((tmp_path / "report.json").read_text())
    assert len(report["rows"]) == 1 and report["rows"][0]["codec"] == "h264"
    assert "summary" in report


def test_missing_binaries_say_how_to_build_them(tmp_path):
    with pytest.raises(FileNotFoundError) as e:
        find_binaries(tmp_path)
    assert "cargo build" in str(e.value)
