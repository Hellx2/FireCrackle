"""Measure what an encode looks like (codec, bitrate, keyframe interval, B-frames, ...), to calibrate the
YouTube-like profiles against real YouTube-delivered files you downloaded yourself.

CLI: python calibrate.py FILE [FILE ...]
"""

import json
import statistics
import subprocess
import sys
from pathlib import Path

from profiles import LADDER


class CalibrateError(Exception):
    pass


def _ffprobe(args, path: Path):
    try:
        out = subprocess.run(["ffprobe", "-v", "error", *args, "-of", "json", str(path)], capture_output=True, text=True)
    except OSError as e:
        raise CalibrateError(f"could not run ffprobe (is ffmpeg installed?): {e}")
    if out.returncode != 0:
        raise CalibrateError(f"ffprobe could not read {path}: {out.stderr.strip()}")
    return json.loads(out.stdout)


def analyze(path) -> dict:
    path = Path(path)
    if not path.is_file():
        raise FileNotFoundError(path)
    info = _ffprobe(
        ["-select_streams", "v:0", "-show_entries",
         "stream=codec_name,profile,level,width,height,pix_fmt,color_range,r_frame_rate:format=duration,size"],
        path,
    )
    if not info.get("streams"):
        raise CalibrateError(f"{path} has no video stream")
    s = info["streams"][0]
    duration = float(info["format"].get("duration") or 0)
    size = int(info["format"].get("size") or path.stat().st_size)
    num, den = (s.get("r_frame_rate") or "0/1").split("/")
    fps = int(num) / int(den) if int(den) else 0.0

    frames = _ffprobe(["-select_streams", "v:0", "-show_entries", "frame=key_frame,pict_type"], path).get("frames", [])
    key_idx = [i for i, f in enumerate(frames) if int(f.get("key_frame", 0)) == 1]
    gaps = [b - a for a, b in zip(key_idx, key_idx[1:])]
    n_b = sum(1 for f in frames if f.get("pict_type") == "B")
    return {
        "codec": s.get("codec_name"),
        "profile": s.get("profile"),
        "level": s.get("level"),
        "width": int(s["width"]),
        "height": int(s["height"]),
        "fps": fps,
        "pix_fmt": s.get("pix_fmt"),
        "color_range": s.get("color_range", "unknown"),
        "frames": len(frames),
        "duration_s": duration,
        "bitrate_kbps": (size * 8 / duration / 1000) if duration > 0 else None,
        "keyframe_interval": {
            "median": int(statistics.median(gaps)) if gaps else None,
            "min": min(gaps) if gaps else None,
            "max": max(gaps) if gaps else None,
        },
        "b_frame_ratio": (n_b / len(frames)) if frames else 0.0,
    }


def nearest_rung(stats: dict) -> dict:
    """The ladder rung closest in height, and the codec, so a measurement can be filed under the right range."""
    height = min(LADDER, key=lambda h: (abs(h - stats["height"]), h))
    return {"codec": stats["codec"], "height": height}


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if not argv:
        print("Usage: calibrate.py FILE [FILE ...]", file=sys.stderr)
        return 2
    rc = 0
    print(f"{'file':28} {'codec':6} {'profile':10} {'size':10} {'fps':>6} {'kbps':>8} {'gop':>5} {'b%':>5} {'pix_fmt':9} {'range':8} rung")
    for p in argv:
        try:
            s = analyze(p)
        except (FileNotFoundError, CalibrateError) as e:
            print(f"error: {e}", file=sys.stderr)
            rc = 1
            continue
        r = nearest_rung(s)
        gop = s["keyframe_interval"]["median"]
        print(f"{Path(p).name[:28]:28} {s['codec']:6} {str(s['profile'])[:10]:10} {s['width']}x{s['height']:<5} "
              f"{s['fps']:6.2f} {s['bitrate_kbps'] or 0:8.0f} {gop if gop is not None else '-':>5} "
              f"{s['b_frame_ratio'] * 100:5.0f} {str(s['pix_fmt']):9} {str(s['color_range']):8} {r['codec']}/{r['height']}p")
    return rc


if __name__ == "__main__":
    sys.exit(main())
