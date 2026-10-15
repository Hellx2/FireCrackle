import json
import random
import shutil
import subprocess

import pytest

import degrade
from degrade import DegradeError, run_chain
from profiles import Source, sample_chain

pytestmark = pytest.mark.skipif(
    shutil.which("ffmpeg") is None or shutil.which("ffprobe") is None, reason="ffmpeg/ffprobe not installed"
)

FRAMES = 30


@pytest.fixture(scope="module")
def clip(tmp_path_factory):
    p = tmp_path_factory.mktemp("clip") / "src.mp4"
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=30",
         "-frames:v", str(FRAMES), "-c:v", "libx264", "-crf", "18", "-pix_fmt", "yuv420p", str(p)],
        check=True,
    )
    return p


def probe(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-count_frames", "-select_streams", "v:0",
         "-show_entries", "stream=codec_name,width,height,pix_fmt,nb_read_frames", "-of", "json", str(path)],
        check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)["streams"][0]


def spec(codec, height=360, recorder=False, seed=1):
    return sample_chain(random.Random(seed), Source(1280, 720, 30.0), codec=codec, height=height,
                        with_recorder=recorder, seed=seed)


@pytest.mark.parametrize("codec,name", [("h264", "h264"), ("vp9", "vp9"), ("av1", "av1")])
def test_each_codec_chain_produces_the_right_stream(clip, tmp_path, codec, name):
    res = run_chain(clip, tmp_path, spec(codec))
    s = probe(res.output)
    assert s["codec_name"] == name
    assert s["height"] == 360 and s["width"] == 640
    assert s["pix_fmt"] == "yuv420p"
    assert int(s["nb_read_frames"]) == FRAMES, "the frame count must survive, or label joins break"


def test_the_recorder_stage_runs_first_and_its_file_is_cleaned_up(clip, tmp_path):
    res = run_chain(clip, tmp_path, spec("h264", recorder=True))
    assert len(res.manifest["commands"]) == 2
    assert int(probe(res.output)["nb_read_frames"]) == FRAMES
    assert not list(tmp_path.glob("*.mkv")), "the intermediate recorder file is removed"


def test_a_manifest_is_written_next_to_the_output(clip, tmp_path):
    res = run_chain(clip, tmp_path, spec("h264", seed=9))
    assert res.manifest_path.exists()
    m = json.loads(res.manifest_path.read_text())
    assert m["seed"] == 9 and m["codec"] == "h264" and m["height"] == 360
    assert m["output"] == res.output.name
    assert "ffmpeg version" in m["ffmpeg"]
    assert m["ranges_are_assumptions"] is True
    assert all(isinstance(c, list) and c[0] == "ffmpeg" for c in m["commands"])


# ---------- failures ----------

def test_missing_input_is_a_file_not_found(tmp_path):
    with pytest.raises(FileNotFoundError):
        run_chain(tmp_path / "nope.mp4", tmp_path, spec("h264"))


def test_a_corrupt_input_is_a_degrade_error_with_the_reason(tmp_path):
    bad = tmp_path / "bad.mp4"
    bad.write_text("this is not a video")
    with pytest.raises(DegradeError) as e:
        run_chain(bad, tmp_path, spec("h264"))
    assert "ffmpeg" in str(e.value).lower()


def test_a_missing_ffmpeg_names_the_problem(clip, tmp_path):
    with pytest.raises(DegradeError) as e:
        run_chain(clip, tmp_path, spec("h264"), ffmpeg="ffmpeg-that-does-not-exist")
    assert "ffmpeg" in str(e.value).lower()


def test_a_failed_run_leaves_no_half_written_output(tmp_path):
    bad = tmp_path / "bad.mp4"
    bad.write_text("garbage")
    out = tmp_path / "out"
    with pytest.raises(DegradeError):
        run_chain(bad, out, spec("h264"))
    assert not list(out.glob("*.mp4")) if out.exists() else True
