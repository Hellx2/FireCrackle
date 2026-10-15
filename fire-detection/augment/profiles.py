"""YouTube-like degradation profiles: the ladder, the bitrate ranges, and the ffmpeg arguments.

Pure functions only (no ffmpeg is run here). The facts and the assumptions behind the numbers are in
YOUTUBE_ENCODING.md. Every default range is an ASSUMPTION until it is replaced by measurements from real
YouTube-delivered files (see calibrate.py and profiles_calibrated.json).
"""

import json
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

LADDER: List[int] = [2160, 1440, 1080, 720, 480, 360, 240, 144]
CODECS = ("h264", "vp9", "av1")
# YouTube's H.264 ladder stops at 1080p; VP9 and AV1 go higher.
MAX_HEIGHT = {"h264": 1080, "vp9": 2160, "av1": 2160}
SCALERS = ("bicubic", "lanczos", "bilinear")
MIN_STRIP_WIDTH = 480  # the sync strip needs this much video width (sync/PLAN.md)

Ranges = Dict[str, Dict[int, Tuple[int, int]]]

# kbps for frame rates up to 30 fps. ASSUMPTION: deliberately wide, anchored on the published points listed in
# YOUTUBE_ENCODING.md (1080p: VP9 2646 and 6427 kbps, H.264 5055 and 5899 kbps; VP9 ~15% and AV1 ~45% below H.264).
DEFAULT_RANGES: Ranges = {
    "h264": {1080: (3000, 9000), 720: (1500, 5000), 480: (700, 2500), 360: (350, 1200), 240: (180, 600), 144: (70, 300)},
    "vp9": {2160: (7000, 26000), 1440: (3000, 10000), 1080: (1500, 5000), 720: (800, 3200), 480: (400, 1600),
            360: (200, 900), 240: (100, 450), 144: (50, 220)},
    "av1": {2160: (5000, 19000), 1440: (2000, 7500), 1080: (1000, 3800), 720: (500, 2400), 480: (250, 1200),
            360: (150, 700), 240: (80, 350), 144: (40, 170)},
}


@dataclass(frozen=True)
class Source:
    width: int
    height: int
    fps: float


@dataclass(frozen=True)
class RecorderSpec:
    crf: int
    preset: str = "veryfast"


@dataclass(frozen=True)
class YouTubeSpec:
    codec: str
    width: int
    height: int
    bitrate_kbps: int
    gop: int
    scaler: str
    fps: float


@dataclass(frozen=True)
class ChainSpec:
    source: Source
    recorder: Optional[RecorderSpec]
    youtube: YouTubeSpec
    seed: int


def _table(codec: str, ranges: Optional[Ranges]) -> Dict[int, Tuple[int, int]]:
    if codec not in CODECS:
        raise ValueError(f"unknown codec {codec!r}; expected one of {CODECS}")
    table = dict(DEFAULT_RANGES[codec])
    if ranges:
        table.update(ranges.get(codec, {}))
    return table


def bitrate_range(codec: str, height: int, fps: float, ranges: Optional[Ranges] = None) -> Tuple[int, int]:
    """(low, high) kbps for this codec and rung; 48 fps and above cost 1.5x."""
    table = _table(codec, ranges)
    if height not in table:
        raise ValueError(f"no {codec} range for height {height}")
    lo, hi = table[height]
    if fps > 40:
        lo, hi = round(lo * 1.5), round(hi * 1.5)
    return lo, hi


def _is_calibrated(codec: str, height: int, ranges: Optional[Ranges]) -> bool:
    return bool(ranges) and height in ranges.get(codec, {})


def sample_chain(
    rng: random.Random,
    source: Source,
    codec: Optional[str] = None,
    height: Optional[int] = None,
    with_recorder: Optional[bool] = None,
    ranges: Optional[Ranges] = None,
    seed: int = 0,
) -> ChainSpec:
    if source.width <= 0 or source.height <= 0 or source.fps <= 0:
        raise ValueError(f"invalid source {source}")
    if codec is None:
        codec = rng.choices(CODECS, weights=[4, 4, 2])[0]
    if codec not in CODECS:
        raise ValueError(f"unknown codec {codec!r}; expected one of {CODECS}")
    allowed = [h for h in LADDER if h <= min(source.height, MAX_HEIGHT[codec])]
    if not allowed:
        raise ValueError(f"source height {source.height} is below the lowest rung ({LADDER[-1]})")
    if height is None:
        height = rng.choice(allowed)
    elif height not in allowed:
        raise ValueError(f"{codec} cannot serve {height}p from a {source.height}p source (allowed: {allowed})")

    width = max(2, int(round(height * source.width / source.height / 2.0)) * 2)
    lo, hi = bitrate_range(codec, height, source.fps, ranges)
    bitrate = rng.randint(lo, hi)
    gop = round(rng.uniform(2.0, 5.0) * source.fps)  # keyframe every 2 to 5 s (ASSUMPTION)
    scaler = rng.choice(SCALERS)
    if with_recorder is None:
        with_recorder = rng.random() < 0.7
    recorder = RecorderSpec(crf=rng.randint(16, 23)) if with_recorder else None
    return ChainSpec(
        source=source,
        recorder=recorder,
        youtube=YouTubeSpec(codec, width, height, bitrate, gop, scaler, source.fps),
        seed=seed,
    )


def output_extension(codec: str) -> str:
    return {"h264": ".mp4", "vp9": ".webm", "av1": ".mp4"}[codec]


_HEAD = ["ffmpeg", "-y", "-loglevel", "error", "-nostdin"]
_BT709 = ["-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709"]


def _codec_args(y: YouTubeSpec) -> List[str]:
    b = y.bitrate_kbps
    if y.codec == "h264":
        return ["-c:v", "libx264", "-preset", "medium", "-profile:v", "high", "-b:v", f"{b}k",
                "-maxrate", f"{round(b * 1.5)}k", "-bufsize", f"{b * 3}k", "-g", str(y.gop), "-bf", "2",
                "-pix_fmt", "yuv420p", *_BT709]
    if y.codec == "vp9":
        return ["-c:v", "libvpx-vp9", "-b:v", f"{b}k", "-deadline", "good", "-cpu-used", "4", "-row-mt", "1",
                "-g", str(y.gop), "-pix_fmt", "yuv420p", *_BT709]
    return ["-c:v", "libsvtav1", "-b:v", f"{b}k", "-preset", "8", "-g", str(y.gop), "-pix_fmt", "yuv420p", *_BT709]


def stage_args(spec: ChainSpec, input_path, intermediate_path, output_path) -> List[List[str]]:
    """The ffmpeg commands of the chain, in order. `-fps_mode passthrough` keeps the frame count (labels join on it)."""
    commands: List[List[str]] = []
    src = str(input_path)
    if spec.recorder is not None:
        commands.append(
            _HEAD + ["-i", src, "-fps_mode", "passthrough", "-c:v", "libx264", "-preset", spec.recorder.preset,
                     "-crf", str(spec.recorder.crf), "-pix_fmt", "yuv420p", "-an", str(intermediate_path)]
        )
        src = str(intermediate_path)
    y = spec.youtube
    commands.append(
        _HEAD + ["-i", src, "-fps_mode", "passthrough", "-vf", f"scale={y.width}:{y.height}:flags={y.scaler}",
                 *_codec_args(y), "-an", str(output_path)]
    )
    return commands


def manifest(spec: ChainSpec, ranges: Optional[Ranges] = None) -> dict:
    y = spec.youtube
    calibrated = _is_calibrated(y.codec, y.height, ranges)
    return {
        "seed": spec.seed,
        "codec": y.codec,
        "width": y.width,
        "height": y.height,
        "bitrate_kbps": y.bitrate_kbps,
        "gop": y.gop,
        "scaler": y.scaler,
        "fps": y.fps,
        "source": {"width": spec.source.width, "height": spec.source.height, "fps": spec.source.fps},
        "recorder_crf": spec.recorder.crf if spec.recorder else None,
        "ranges_are_assumptions": not calibrated,
        "strip_decodable": y.width >= MIN_STRIP_WIDTH,
    }


def load_ranges(path) -> Ranges:
    """Reads a calibrated override file: {"h264": {"1080": [lo, hi], ...}, ...}. Rejects anything malformed."""
    try:
        raw = json.loads(Path(path).read_text())
    except (OSError, ValueError) as e:
        raise ValueError(f"cannot read ranges file {path}: {e}")
    if not isinstance(raw, dict):
        raise ValueError("ranges file must be a JSON object")
    out: Ranges = {}
    for codec, rungs in raw.items():
        if codec not in CODECS:
            raise ValueError(f"unknown codec {codec!r} in ranges file")
        if not isinstance(rungs, dict):
            raise ValueError(f"{codec}: expected an object of rung -> [low, high]")
        out[codec] = {}
        for rung, pair in rungs.items():
            try:
                height = int(rung)
            except ValueError:
                raise ValueError(f"{codec}: {rung!r} is not a height")
            if height not in LADDER:
                raise ValueError(f"{codec}: {height} is not a ladder rung {LADDER}")
            if not (isinstance(pair, list) and len(pair) == 2 and all(isinstance(v, (int, float)) for v in pair)):
                raise ValueError(f"{codec} {height}: expected [low, high]")
            lo, hi = int(pair[0]), int(pair[1])
            if not 0 < lo < hi:
                raise ValueError(f"{codec} {height}: need 0 < low < high, got {pair}")
            out[codec][height] = (lo, hi)
    return out
