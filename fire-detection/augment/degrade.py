"""Run a degradation chain (recorder -> YouTube-like re-encode) with ffmpeg and record exactly what was done.

CLI: python degrade.py INPUT OUT_DIR [--seed N] [--codec h264|vp9|av1] [--height H] [--ranges FILE]
"""

import argparse
import json
import random
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import profiles
from profiles import ChainSpec, Ranges, Source, manifest, output_extension, sample_chain, stage_args


class DegradeError(Exception):
    pass


@dataclass
class Result:
    output: Path
    manifest_path: Path
    manifest: dict


def _ffmpeg_version(ffmpeg: str) -> str:
    try:
        out = subprocess.run([ffmpeg, "-version"], capture_output=True, text=True)
    except OSError as e:
        raise DegradeError(f"ffmpeg executable not found or not runnable ({ffmpeg}): {e}")
    return (out.stdout.splitlines() or ["ffmpeg version unknown"])[0]


def run_chain(input_path, out_dir, spec: ChainSpec, ffmpeg: str = "ffmpeg", ranges: Optional[Ranges] = None) -> Result:
    input_path = Path(input_path)
    if not input_path.is_file():
        raise FileNotFoundError(input_path)
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    y = spec.youtube
    name = f"{input_path.stem}.{y.codec}-{y.height}p-{y.bitrate_kbps}k-s{spec.seed}"
    output = out_dir / (name + output_extension(y.codec))
    intermediate = out_dir / f".{name}.rec.mkv"

    version = _ffmpeg_version(ffmpeg)
    commands = stage_args(spec, input_path, intermediate, output)
    commands = [[ffmpeg] + c[1:] for c in commands]
    try:
        for cmd in commands:
            try:
                done = subprocess.run(cmd, capture_output=True, text=True)
            except OSError as e:
                raise DegradeError(f"ffmpeg executable not found or not runnable ({ffmpeg}): {e}")
            if done.returncode != 0:
                tail = "\n".join(done.stderr.strip().splitlines()[-5:])
                raise DegradeError(f"ffmpeg failed (exit {done.returncode}): {tail}")
    except DegradeError:
        output.unlink(missing_ok=True)  # never leave a half-written file that looks like a result
        raise
    finally:
        intermediate.unlink(missing_ok=True)

    m = manifest(spec, ranges)
    m.update({"input": input_path.name, "output": output.name, "ffmpeg": version, "commands": commands})
    manifest_path = out_dir / (output.name + ".manifest.json")
    manifest_path.write_text(json.dumps(m, indent=2))
    return Result(output, manifest_path, m)


def _probe_source(path: Path) -> Source:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,r_frame_rate",
         "-of", "json", str(path)],
        capture_output=True, text=True,
    )
    if out.returncode != 0:
        raise DegradeError(f"ffprobe could not read {path}: {out.stderr.strip()}")
    s = json.loads(out.stdout)["streams"][0]
    num, den = s["r_frame_rate"].split("/")
    return Source(int(s["width"]), int(s["height"]), int(num) / int(den))


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("input")
    ap.add_argument("out_dir")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--codec", choices=profiles.CODECS)
    ap.add_argument("--height", type=int)
    ap.add_argument("--ranges", help="calibrated ranges JSON (overrides the assumptions)")
    a = ap.parse_args(argv)
    try:
        ranges = profiles.load_ranges(a.ranges) if a.ranges else None
        src = _probe_source(Path(a.input))
        spec = sample_chain(random.Random(a.seed), src, codec=a.codec, height=a.height, ranges=ranges, seed=a.seed)
        res = run_chain(a.input, a.out_dir, spec, ranges=ranges)
    except (DegradeError, ValueError, FileNotFoundError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    print(res.output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
