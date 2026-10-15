"""What does a recording (or label log) actually cover, and what is missing?

Reads either a label log from the mod or the joined output of `fire-decode --labels ... --out joined.jsonl`
(the latter is what a real recording gives: only the frames that are in the video, with their label rows),
and reports fire-on-screen share, sources, distances, box sizes, visibility and how often each of the
32 texture rows was seen per (source, sheet, mirrored), then lists the gaps.

CLI: python coverage_report.py FILE.jsonl [--min-row-count 20]
"""

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

DIST_BUCKETS = [f"{lo}-{lo + 5}" for lo in range(0, 40, 5)] + ["40+"]
BOX_BUCKETS = ["<0.1%", "0.1-1%", "1-10%", ">=10%"]
VIS_BUCKETS = ["<0.1", "0.1-0.5", "0.5-0.9", ">=0.9"]
EXPECTED = {
    "block": [(0, False), (0, True), (1, False), (1, True)],
    "entity": [(0, False), (0, True), (1, False), (1, True)],
    "overlay": [(1, True)],
}
MIN_FIRE_PCT = 40.0
MIN_NEGATIVE_PCT = 5.0
MIN_DISTINCT_BLOCKS = 10
MIN_DISTINCT_ENTITIES = 5


def load_rows(path):
    """Returns (label rows in the current schema, stats). Joined rows are unwrapped; unmatched ones are counted."""
    stats = {"lines": 0, "bad_lines": 0, "old_schema": 0, "unlabelled": 0}
    rows = []
    with open(path, encoding="utf8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            stats["lines"] += 1
            try:
                r = json.loads(line)
            except ValueError:
                stats["bad_lines"] += 1
                continue
            if not isinstance(r, dict):
                stats["bad_lines"] += 1
                continue
            if "label" in r:  # joined output of fire-decode
                r = r["label"]
                if r is None:
                    stats["unlabelled"] += 1
                    continue
            elif "fires" not in r:
                stats["bad_lines"] += 1
                continue
            if "w" not in r or "h" not in r or any("parts" not in s for s in r["fires"]):
                stats["old_schema"] += 1  # older log format: no size or no parts, so it cannot be used
                continue
            rows.append(r)
    return rows, stats


def _dist_bucket(d):
    return "40+" if d >= 40 else f"{int(d // 5) * 5}-{int(d // 5) * 5 + 5}"


def _box_bucket(box, w, h):
    ratio = max(0.0, (box[2] - box[0]) * (box[3] - box[1])) / (w * h)
    return "<0.1%" if ratio < 0.001 else "0.1-1%" if ratio < 0.01 else "1-10%" if ratio < 0.1 else ">=10%"


def _vis_bucket(v):
    return "<0.1" if v < 0.1 else "0.1-0.5" if v < 0.5 else "0.5-0.9" if v < 0.9 else ">=0.9"


def coverage(rows, min_row_count=20):
    frames = len(rows)
    fire_frames = sum(1 for r in rows if r["fire"])
    entries = Counter()
    blocks, entities = set(), set()
    dist, box, vis = Counter(), Counter(), Counter()
    row_hits = defaultdict(Counter)  # (src, sheet, mirror) -> row -> count
    for r in rows:
        for s in r["fires"]:
            entries[s["src"]] += 1
            if s["src"] == "block":
                blocks.add(tuple(s["pos"]))
            elif s["src"] == "entity":
                entities.add(s["id"])
            if s["src"] != "overlay":  # the overlay is a separate regime: fixed distance, huge box
                dist[_dist_bucket(s["dist"])] += 1
                box[_box_bucket(s["box"], r["w"], r["h"])] += 1
                vis[_vis_bucket(s["vis"])] += 1
            for p in s["parts"]:
                row_hits[(s["src"], p["sheet"], p["mirror"])][p["row"]] += 1

    rows_info = {}
    for key, hits in row_hits.items():
        rows_info[key] = {
            "rows_covered": len(hits),
            "entries": sum(hits.values()),
            "min_count": min(hits.values()),
            "missing": [i for i in range(32) if i not in hits],
        }

    cov = {
        "frames": frames,
        "fire_frames": fire_frames,
        "fire_pct": round(100 * fire_frames / frames, 1) if frames else 0.0,
        "entries": dict(entries),
        "distinct_blocks": len(blocks),
        "distinct_entities": len(entities),
        "distance_buckets": {b: dist.get(b, 0) for b in DIST_BUCKETS},
        "box_area": {b: box.get(b, 0) for b in BOX_BUCKETS},
        "vis_buckets": {b: vis.get(b, 0) for b in VIS_BUCKETS},
        "rows": rows_info,
    }
    cov["gaps"] = _gaps(cov, min_row_count)
    return cov


def _gaps(c, min_row_count):
    gaps = []
    if c["frames"] == 0:
        return ["no usable frames"]
    negatives_pct = 100 - c["fire_pct"]
    if c["fire_pct"] < MIN_FIRE_PCT:
        gaps.append(f"only {c['fire_pct']}% of frames have fire; aim for at least {MIN_FIRE_PCT:.0f}% so the capture is mostly fire")
    if negatives_pct < MIN_NEGATIVE_PCT:
        gaps.append(f"only {negatives_pct:.1f}% fire-free frames; hard negatives (lava, torches, glowstone, orange scenes) are needed to measure false positives")
    for src, label in (("block", "fire-block fire"), ("entity", "burning-entity fire (mobs on fire)"),
                       ("overlay", "first-person on-fire overlay")):
        if not c["entries"].get(src):
            gaps.append(f"no {label} at all")
    names = {False: "normal", True: "mirrored"}
    for src, combos in EXPECTED.items():
        if not c["entries"].get(src):
            continue
        for sheet, mirror in combos:
            info = c["rows"].get((src, sheet, mirror))
            what = f"{src} sheet {sheet} {names[mirror]}"
            if info is None:
                gaps.append(f"{what}: never seen")
            elif info["rows_covered"] < 32 or info["min_count"] < min_row_count:
                gaps.append(f"{what}: rows covered {info['rows_covered']}/32, rarest row seen {info['min_count']} times (want {min_row_count}+)")
    if c["entries"].get("block") or c["entries"].get("entity"):
        d = c["distance_buckets"]
        if sum(v for k, v in d.items() if k not in ("0-5", "5-10", "10-15", "15-20")) == 0:
            gaps.append("no fire farther than 20 blocks (distance)")
        if d["0-5"] == 0:
            gaps.append("no close fire (under 5 blocks)")
        if c["box_area"]["<0.1%"] == 0:
            gaps.append("no small fire (boxes under 0.1% of the frame, i.e. far away)")
        if c["vis_buckets"]["<0.1"] + c["vis_buckets"]["0.1-0.5"] == 0:
            gaps.append("no partly hidden fire (occlusion by walls or mobs)")
        if c["entries"].get("block") and c["distinct_blocks"] < MIN_DISTINCT_BLOCKS:
            gaps.append(f"only {c['distinct_blocks']} distinct fire blocks (aim for {MIN_DISTINCT_BLOCKS}+: other places and backgrounds)")
        if c["entries"].get("entity") and c["distinct_entities"] < MIN_DISTINCT_ENTITIES:
            gaps.append(f"only {c['distinct_entities']} distinct burning entities (aim for {MIN_DISTINCT_ENTITIES}+: other mob types and sizes)")
    return gaps


def format_report(c):
    lines = [f"frames: {c['frames']}   with fire: {c['fire_frames']} ({c['fire_pct']}%)",
             f"entries: {c['entries']}   distinct fire blocks: {c['distinct_blocks']}   distinct burning entities: {c['distinct_entities']}",
             "distance (blocks): " + "  ".join(f"{k}:{v}" for k, v in c["distance_buckets"].items()),
             "box size (share of frame): " + "  ".join(f"{k}:{v}" for k, v in c["box_area"].items()),
             "visibility: " + "  ".join(f"{k}:{v}" for k, v in c["vis_buckets"].items()),
             "texture rows seen per (source, sheet, mirrored):"]
    for (src, sheet, mirror), i in sorted(c["rows"].items()):
        lines.append(f"  {src:7} sheet {sheet} {'mirrored' if mirror else 'normal  '}: {i['rows_covered']:2}/32 rows, "
                     f"{i['entries']} entries, rarest row {i['min_count']}x")
    lines.append("gaps:" if c["gaps"] else "gaps: none")
    lines.extend(f"  - {g}" for g in c["gaps"])
    return "\n".join(lines)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("file", nargs="?")
    ap.add_argument("--min-row-count", type=int, default=20, help="wanted sightings of the rarest texture row")
    try:
        a = ap.parse_args(argv)
    except SystemExit:
        return 2
    if not a.file:
        print("Usage: coverage_report.py FILE.jsonl [--min-row-count N]", file=sys.stderr)
        return 2
    try:
        rows, stats = load_rows(Path(a.file))
    except OSError as e:
        print(f"error: cannot read {a.file}: {e}", file=sys.stderr)
        return 1
    print(f"read {stats['lines']} lines: {len(rows)} usable, {stats['unlabelled']} without a label row, "
          f"{stats['old_schema']} in an older format, {stats['bad_lines']} unreadable")
    print(format_report(coverage(rows, a.min_row_count)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
