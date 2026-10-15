"""Does temporal flicker find small / far fire? See core/DETECT_PLAN.md round 6.

    python bench/flicker_analysis.py data/joined/flicker5.csv
"""
import statistics
import sys

import blob_analysis as ba
import presence_analysis as pa

FEATURES = ["t30_tot", "t30_in6", "t30_big", "t60_tot", "t60_in6", "t60_big"]
CLASSES = ["bignear", "small", "far", "smallfar", "tiny"]


def table(rows, title):
    by_n = {int(r["n"]): r for r in rows}
    neg = [int(r["n"]) for r in rows if not r["fire"]]
    sets = ba.pos_sets(rows)
    print(f"\n== {title}: no-fire frames {len(neg)}; fire frames per class: "
          + ", ".join(f"{k} {len(sets[k])}" for k in CLASSES))
    print(f"  {'feature':<9}" + "".join(f"{k:>10}" for k in CLASSES) + "   (AUC vs no-fire frames)")
    for f in FEATURES:
        cells = []
        for k in CLASSES:
            if len(sets[k]) < 8 or len(neg) < 8:
                cells.append(f"{'-':>10}")
                continue
            a = pa.auc([by_n[n][f] for n in sets[k]], [by_n[n][f] for n in neg])
            cells.append(f"{a:10.3f}")
        print(f"  {f:<9}" + "".join(cells))


def main(argv):
    rows = pa.load(argv[1])
    print(f"rows={len(rows)}  fire frames={sum(1 for r in rows if r['fire'])}")
    bgs = sorted(r["bg"] for r in rows)
    q = [bgs[int(len(bgs) * p)] for p in (0.25, 0.5, 0.75)]
    print(f"camera motion (bg) quartiles: {q[0]:.2f} {q[1]:.2f} {q[2]:.2f}; gap frames median {statistics.median(r['gap'] for r in rows):.0f}")
    table(rows, "all frames")
    still = [r for r in rows if r["bg"] <= q[0]]
    table(still, f"still camera (bg <= {q[0]:.2f}, lowest quarter)")
    mid = [r for r in rows if r["bg"] <= q[1]]
    table(mid, f"calm camera (bg <= {q[1]:.2f}, lowest half)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
