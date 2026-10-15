"""Offline analysis of fire-features CSV: which feature separates fire from no fire.

    python bench/presence_analysis.py data/joined/features10.csv [--min-area 1000]
"""
import csv
import sys

MISSING = ("", None)


def auc(pos, neg):
    """P(pos > neg) + 0.5 P(pos == neg); None if a side is empty. O(n log n) rank-sum."""
    if not pos or not neg:
        return None
    vals = sorted([(v, 1) for v in pos] + [(v, 0) for v in neg])
    rank_sum = 0.0
    i = 0
    while i < len(vals):
        j = i
        while j < len(vals) and vals[j][0] == vals[i][0]:
            j += 1
        avg_rank = (i + 1 + j) / 2.0
        rank_sum += avg_rank * sum(l for _, l in vals[i:j])
        i = j
    n_pos, n_neg = len(pos), len(neg)
    return (rank_sum - n_pos * (n_pos + 1) / 2.0) / (n_pos * n_neg)


def best_threshold(pos, neg):
    """Threshold t (score >= t means fire) maximising balanced accuracy; returns (t, balanced_accuracy)."""
    if not pos or not neg:
        return None
    best = None
    for t in sorted(set(pos) | set(neg)):
        tpr = sum(1 for v in pos if v >= t) / len(pos)
        tnr = sum(1 for v in neg if v < t) / len(neg)
        bal = (tpr + tnr) / 2.0
        if best is None or bal > best[1] + 1e-12:
            best = (t, bal)
    return best


def load(path):
    rows = []
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            row = {}
            for k, v in r.items():
                row[k] = None if v in MISSING else float(v)
            rows.append(row)
    return rows


def split(rows, min_area):
    """(negatives, positives): no fire at all, versus visible fire area >= min_area. Tiny fires are in neither."""
    neg = [r for r in rows if not r["fire"]]
    pos = [r for r in rows if r["fire"] and r["area"] >= min_area]
    return neg, pos


FEATURES = ["cnt", "strict", "bright", "blobs20", "largest", "inbig50", "rough", "roughbig"]


def main(argv):
    path = argv[1]
    min_area = float(argv[argv.index("--min-area") + 1]) if "--min-area" in argv else 1000.0
    rows = load(path)
    neg, pos = split(rows, min_area)
    print(f"frames={len(rows)} no-fire={len(neg)} fire(area>={min_area:g})={len(pos)}")
    print(f"{'feature':<12}{'AUC':>8}{'best bal.acc':>14}{'thr':>12}")
    results = []
    for prefix in ("r_", "u_"):
        for name in FEATURES:
            key = prefix + name
            p = [r[key] for r in pos if r.get(key) is not None]
            n = [r[key] for r in neg if r.get(key) is not None]
            a = auc(p, n)
            b = best_threshold(p, n)
            results.append((a, key, b))
    for g in ("mean_luma", "dark"):
        p = [r[g] for r in pos]
        n = [r[g] for r in neg]
        results.append((auc(p, n), g, best_threshold(p, n)))
    for a, key, b in sorted(results, key=lambda t: -(abs((t[0] or 0.5) - 0.5))):
        print(f"{key:<12}{a:8.3f}{b[1]:14.3f}{b[0]:12.4g}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
