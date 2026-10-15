"""How well does the sync strip survive YouTube-like compression?

Generates a stamped synthetic video (Rust `stamp-test-video`), degrades it with sampled chains (profiles.py /
degrade.py), decodes the results with `fire-decode`, and reports, per codec and rung, how many frames still decode
to the right frame id and tick. Build the Rust tools first: `cargo build --release` in core/.

CLI: python robustness.py [--frames 120] [--size 1920x1080] [--codecs h264,vp9,av1] [--heights 1080,720,...]
                          [--seeds 2] [--out DIR] [--keep]
"""

import argparse
import json
import random
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

import profiles
from degrade import DegradeError, run_chain
from profiles import ChainSpec, Source, manifest, sample_chain


def find_binaries(core_dir=None):
    core = Path(core_dir) if core_dir else Path(__file__).resolve().parent.parent / "core"
    release = core / "target" / "release"
    ext = ".exe" if sys.platform == "win32" else ""
    decode, gen = release / f"fire-decode{ext}", release / f"stamp-test-video{ext}"
    if not (decode.is_file() and gen.is_file()):
        raise FileNotFoundError(f"fire-decode / stamp-test-video not found in {release}; run `cargo build --release` in {core}")
    return decode, gen


def build_source(gen, out, frames, size, fps):
    out = Path(out)
    done = subprocess.run(
        [str(gen), str(out), "--frames", str(frames), "--size", f"{size[0]}x{size[1]}", "--fps", str(fps)],
        capture_output=True, text=True,
    )
    if done.returncode != 0:
        raise RuntimeError(f"stamp-test-video failed: {done.stderr.strip()}")
    return out


def decode_accuracy(decode, video, frames):
    """Run fire-decode; `correct` counts frames whose decoded id is the frame index and tick is 1000 + n // 3."""
    done = subprocess.run([str(decode), str(video)], capture_output=True, text=True)
    if done.returncode != 0:
        msg = done.stderr.strip()
        return {"status": "too_narrow" if "narrow" in msg else "error", "message": msg, "frames": 0, "decoded": 0, "correct": 0}
    rows = [json.loads(line) for line in done.stdout.splitlines() if line]
    decoded = sum(1 for r in rows if r["f"] is not None)
    correct = sum(1 for r in rows if r["f"] == r["n"] and r["tick"] == 1000 + r["n"] // 3)
    return {"status": "ok", "frames": len(rows), "decoded": decoded, "correct": correct}


def _base(spec: ChainSpec) -> dict:
    m = manifest(spec)
    return {k: m[k] for k in ("codec", "height", "width", "bitrate_kbps", "recorder_crf", "scaler", "strip_decodable", "seed")}


def run_matrix(source, out_dir, specs, decode, frames, keep=False):
    rows = []
    for spec in specs:
        row = _base(spec)
        try:
            res = run_chain(source, out_dir, spec)
        except (DegradeError, FileNotFoundError) as e:
            row.update({"status": "error", "message": str(e), "frames": 0, "decoded": 0, "correct": 0})
            rows.append(row)
            continue
        row.update(decode_accuracy(decode, res.output, frames))
        if not keep:
            res.output.unlink(missing_ok=True)
        rows.append(row)
    return rows


def summarize(rows):
    groups = defaultdict(list)
    for r in rows:
        groups[(r["codec"], r["height"])].append(r)
    out = []
    for (codec, height), rs in sorted(groups.items(), key=lambda kv: (kv[0][0], -kv[0][1])):
        ok = [r for r in rs if r["status"] == "ok" and r["frames"]]
        out.append({
            "codec": codec, "height": height, "chains": len(rs),
            "ok": len(ok), "too_narrow": sum(r["status"] == "too_narrow" for r in rs),
            "errors": sum(r["status"] == "error" for r in rs),
            "decoded_pct": round(100 * sum(r["decoded"] for r in ok) / max(1, sum(r["frames"] for r in ok)), 2) if ok else None,
            "correct_pct": round(100 * sum(r["correct"] for r in ok) / max(1, sum(r["frames"] for r in ok)), 2) if ok else None,
        })
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--frames", type=int, default=120)
    ap.add_argument("--size", default="1920x1080")
    ap.add_argument("--fps", type=int, default=30)
    ap.add_argument("--codecs", default="h264,vp9,av1")
    ap.add_argument("--heights", default="all")
    ap.add_argument("--seeds", type=int, default=2, help="chains sampled per (codec, rung)")
    ap.add_argument("--out", default="robustness_out")
    ap.add_argument("--keep", action="store_true", help="keep the degraded videos")
    a = ap.parse_args(argv)
    try:
        decode, gen = find_binaries()
        w, h = (int(v) for v in a.size.split("x"))
        codecs = a.codecs.split(",")
        for c in codecs:
            if c not in profiles.CODECS:
                raise ValueError(f"unknown codec {c!r}")
        wanted = None if a.heights == "all" else {int(v) for v in a.heights.split(",")}
        out = Path(a.out)
        out.mkdir(parents=True, exist_ok=True)
        source = build_source(gen, out / "src.mp4", a.frames, (w, h), a.fps)
    except (FileNotFoundError, RuntimeError, ValueError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    src = Source(w, h, float(a.fps))
    specs = []
    for codec in codecs:
        for rung in profiles.LADDER:
            if rung > min(h, profiles.MAX_HEIGHT[codec]) or (wanted is not None and rung not in wanted):
                continue
            for k in range(a.seeds):
                specs.append(sample_chain(random.Random(f"{codec}-{rung}-{k}"), src, codec=codec, height=rung, seed=k))
    rows = run_matrix(source, out, specs, decode, a.frames, keep=a.keep)
    summary = summarize(rows)
    (out / "report.json").write_text(json.dumps({"source": {"width": w, "height": h, "fps": a.fps, "frames": a.frames},
                                                 "rows": rows, "summary": summary}, indent=2))
    print(f"{'codec':6} {'rung':>5} {'chains':>6} {'decoded%':>9} {'correct%':>9}  notes")
    for s in summary:
        notes = []
        if s["too_narrow"]:
            notes.append("strip needs >= 480 px width")
        if s["errors"]:
            notes.append(f"{s['errors']} errors")
        print(f"{s['codec']:6} {s['height']:>4}p {s['chains']:>6} {str(s['decoded_pct']):>9} {str(s['correct_pct']):>9}  {', '.join(notes)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
