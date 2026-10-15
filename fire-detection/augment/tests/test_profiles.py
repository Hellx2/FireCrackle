import json
import random

import pytest

import profiles
from profiles import (
    DEFAULT_RANGES,
    LADDER,
    Source,
    bitrate_range,
    load_ranges,
    manifest,
    output_extension,
    sample_chain,
    stage_args,
)

SRC_1080 = Source(1920, 1080, 60.0)
SRC_720 = Source(1280, 720, 30.0)
SRC_4K = Source(3840, 2160, 30.0)


def chain(source=SRC_1080, seed=0, **kw):
    return sample_chain(random.Random(seed), source, seed=seed, **kw)


# ---------- the ladder ----------

def test_ladder_is_the_standard_rungs():
    assert LADDER == [2160, 1440, 1080, 720, 480, 360, 240, 144]


def test_never_upscales():
    for seed in range(200):
        assert chain(SRC_720, seed).youtube.height <= 720


def test_h264_never_goes_above_1080():
    for seed in range(200):
        c = chain(SRC_4K, seed, codec="h264")
        assert c.youtube.height <= 1080


def test_vp9_and_av1_can_serve_the_top_rungs():
    assert chain(SRC_4K, 1, codec="vp9", height=2160).youtube.height == 2160
    assert chain(SRC_4K, 1, codec="av1", height=1440).youtube.height == 1440


def test_explicit_height_must_be_allowed():
    with pytest.raises(ValueError):
        chain(SRC_4K, 1, codec="h264", height=2160)  # H.264 stops at 1080
    with pytest.raises(ValueError):
        chain(SRC_720, 1, height=1080)  # would upscale
    with pytest.raises(ValueError):
        chain(SRC_1080, 1, height=900)  # not a rung


# ---------- bitrates ----------

def test_sampled_bitrates_stay_in_the_table():
    for codec in ("h264", "vp9", "av1"):
        for seed in range(60):
            c = chain(SRC_1080, seed, codec=codec)
            lo, hi = bitrate_range(codec, c.youtube.height, SRC_1080.fps)
            assert lo <= c.youtube.bitrate_kbps <= hi, (codec, c.youtube.height)


def test_high_frame_rate_costs_half_again():
    lo30, hi30 = bitrate_range("h264", 1080, 30.0)
    lo60, hi60 = bitrate_range("h264", 1080, 60.0)
    assert lo60 == round(lo30 * 1.5) and hi60 == round(hi30 * 1.5)
    assert bitrate_range("h264", 1080, 48.0) == (lo60, hi60)
    assert bitrate_range("h264", 1080, 29.97) == (lo30, hi30)


def test_published_anchor_points_are_inside_the_ranges():
    # one measured 1080p video: VP9 2646 kbps, H.264 5055 kbps (30 fps class)
    lo, hi = bitrate_range("vp9", 1080, 30.0)
    assert lo <= 2646 <= hi
    lo, hi = bitrate_range("h264", 1080, 30.0)
    assert lo <= 5055 <= hi
    # a 1080p60 case: VP9 6427 kbps, AVC 5899 kbps
    lo, hi = bitrate_range("vp9", 1080, 60.0)
    assert lo <= 6427 <= hi
    lo, hi = bitrate_range("h264", 1080, 60.0)
    assert lo <= 5899 <= hi


def test_every_rung_has_a_sane_range_and_newer_codecs_are_not_costlier():
    for codec, table in DEFAULT_RANGES.items():
        for height, (lo, hi) in table.items():
            assert 0 < lo < hi, (codec, height)
    for h in (1080, 720, 480):
        assert DEFAULT_RANGES["av1"][h][1] <= DEFAULT_RANGES["vp9"][h][1] <= DEFAULT_RANGES["h264"][h][1] * 1.0 + 1


# ---------- determinism, geometry, gop ----------

def test_same_seed_same_chain_and_seeds_differ():
    assert chain(SRC_1080, 7) == chain(SRC_1080, 7)
    assert len({chain(SRC_1080, s) for s in range(50)}) > 20


@pytest.mark.parametrize("src", [Source(1920, 1080, 30), Source(1920, 800, 24), Source(1366, 768, 60), Source(2560, 1369, 60)])
def test_dimensions_are_even_and_the_aspect_is_kept(src):
    for seed in range(40):
        y = chain(src, seed).youtube
        assert y.width % 2 == 0 and y.height % 2 == 0
        assert abs(y.width / y.height - src.width / src.height) < 0.03 * (src.width / src.height) + 0.02


def test_keyframe_interval_is_two_to_five_seconds():
    for seed in range(100):
        y = chain(Source(1920, 1080, 30), seed).youtube
        assert 60 <= y.gop <= 150 and isinstance(y.gop, int)


def test_scaler_is_one_of_the_known_filters():
    seen = {chain(SRC_1080, s).youtube.scaler for s in range(80)}
    assert seen <= {"bicubic", "lanczos", "bilinear"} and len(seen) >= 2


def test_recorder_stage_is_optional_and_in_range():
    assert chain(SRC_1080, 1, with_recorder=False).recorder is None
    for seed in range(40):
        r = chain(SRC_1080, seed, with_recorder=True).recorder
        assert r is not None and 16 <= r.crf <= 23


# ---------- ffmpeg arguments ----------

def test_h264_arguments():
    c = chain(SRC_1080, 3, codec="h264", height=720, with_recorder=False)
    cmds = stage_args(c, "in.mp4", "mid.mkv", "out.mp4")
    assert len(cmds) == 1
    cmd = cmds[0]
    assert cmd[0] == "ffmpeg" and "-y" in cmd and "-loglevel" in cmd
    assert cmd[cmd.index("-i") + 1] == "in.mp4" and cmd[-1] == "out.mp4"
    for token in ("libx264", "high", "yuv420p", "bt709", f"{c.youtube.bitrate_kbps}k"):
        assert token in cmd, token
    assert cmd[cmd.index("-bf") + 1] == "2"
    assert cmd[cmd.index("-g") + 1] == str(c.youtube.gop)
    vf = cmd[cmd.index("-vf") + 1]
    assert f"scale={c.youtube.width}:{c.youtube.height}" in vf and f"flags={c.youtube.scaler}" in vf


def test_vp9_and_av1_arguments_and_extensions():
    v = chain(SRC_1080, 3, codec="vp9", height=720, with_recorder=False)
    a = chain(SRC_1080, 3, codec="av1", height=720, with_recorder=False)
    cv = stage_args(v, "in.mp4", "mid.mkv", "out.webm")[0]
    ca = stage_args(a, "in.mp4", "mid.mkv", "out.mp4")[0]
    assert "libvpx-vp9" in cv and "yuv420p" in cv
    assert "libsvtav1" in ca and "yuv420p" in ca
    assert output_extension("h264") == ".mp4"
    assert output_extension("vp9") == ".webm"
    assert output_extension("av1") == ".mp4"


def test_the_recorder_stage_adds_a_first_command_with_its_crf():
    c = chain(SRC_1080, 5, codec="h264", height=720, with_recorder=True)
    cmds = stage_args(c, "in.mp4", "mid.mkv", "out.mp4")
    assert len(cmds) == 2
    first, second = cmds
    assert first[first.index("-i") + 1] == "in.mp4" and first[-1] == "mid.mkv"
    assert first[first.index("-crf") + 1] == str(c.recorder.crf)
    assert second[second.index("-i") + 1] == "mid.mkv" and second[-1] == "out.mp4"


# ---------- manifest ----------

def test_manifest_is_json_and_flags_assumptions_and_the_strip():
    c = chain(SRC_1080, 4, codec="vp9", height=1080)
    m = manifest(c)
    json.dumps(m)  # must be serialisable
    assert m["seed"] == 4 and m["codec"] == "vp9" and m["height"] == 1080
    assert m["ranges_are_assumptions"] is True
    assert m["strip_decodable"] is True
    low = manifest(chain(Source(1920, 1080, 30), 4, codec="h264", height=240))
    assert low["strip_decodable"] is False and low["width"] == 426


# ---------- calibrated overrides ----------

def test_a_calibrated_range_replaces_the_assumption():
    ranges = {"h264": {1080: (100, 200)}}
    assert bitrate_range("h264", 1080, 30.0, ranges) == (100, 200)
    assert bitrate_range("h264", 720, 30.0, ranges) == DEFAULT_RANGES["h264"][720]  # others untouched
    c = chain(Source(1920, 1080, 30), 1, codec="h264", height=1080, ranges=ranges)
    assert 100 <= c.youtube.bitrate_kbps <= 200
    assert manifest(c, ranges)["ranges_are_assumptions"] is False


def test_load_ranges_reads_and_validates(tmp_path):
    good = tmp_path / "good.json"
    good.write_text(json.dumps({"h264": {"1080": [4000, 6000]}, "vp9": {"720": [900, 2000]}}))
    assert load_ranges(good) == {"h264": {1080: (4000, 6000)}, "vp9": {720: (900, 2000)}}
    for name, content in {
        "reversed": {"h264": {"1080": [6000, 4000]}},
        "unknown codec": {"hevc": {"1080": [1, 2]}},
        "not a rung": {"h264": {"900": [1, 2]}},
        "zero": {"h264": {"1080": [0, 5]}},
        "wrong shape": {"h264": {"1080": [1, 2, 3]}},
    }.items():
        bad = tmp_path / "bad.json"
        bad.write_text(json.dumps(content))
        with pytest.raises(ValueError):
            load_ranges(bad)
    broken = tmp_path / "broken.json"
    broken.write_text("{not json")
    with pytest.raises(ValueError):
        load_ranges(broken)


# ---------- invalid input ----------

def test_invalid_inputs():
    with pytest.raises(ValueError):
        chain(SRC_1080, 1, codec="hevc")
    with pytest.raises(ValueError):
        chain(Source(160, 100, 30), 1)  # below the lowest rung
    with pytest.raises(ValueError):
        chain(Source(1920, 1080, 0), 1)
    with pytest.raises(ValueError):
        chain(Source(0, 1080, 30), 1)
    with pytest.raises(ValueError):
        bitrate_range("h264", 999, 30.0)
