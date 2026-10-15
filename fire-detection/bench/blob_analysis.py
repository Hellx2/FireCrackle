"""Blob-shape analysis: does shape separate fire from lava/other orange? See core/DETECT_PLAN.md round 5.

    python bench/blob_analysis.py data/joined/blobs10.csv data/joined/colorfilter10.csv
"""
import sys

import numpy as np

import presence_analysis as pa
import train_patch as tp

FEATURES = ["area", "bw", "bh", "fill", "compact", "lum", "lumstd", "white", "sat", "rough"]


def frame_max(blob_frames, scores, frames):
    """{frame: best blob score, 0.0 for frames without blobs}, for the asked-for frames only."""
    if len(blob_frames) != len(scores):
        raise ValueError("blob_frames and scores differ in length")
    out = {int(f): 0.0 for f in frames}
    for f, s in zip(blob_frames, scores):
        f = int(f)
        if f in out and s > out[f]:
            out[f] = float(s)
    return out


def frame_sets(rows):
    """(no-fire frame numbers, big+near fire frame numbers): fire area >= 10k px^2 and nearest source <= 15 blocks."""
    neg = [int(r["n"]) for r in rows if not r["fire"]]
    pos = [
        int(r["n"])
        for r in rows
        if r["fire"] and r["area"] >= 10000 and r["mind"] is not None and r["mind"] <= 15
    ]
    return neg, pos


def pos_sets(rows):
    """Frame numbers of fire frames per difficulty class (areas in px^2, distance in blocks)."""
    out = {"bignear": [], "small": [], "far": [], "smallfar": [], "tiny": []}
    for r in rows:
        if not r["fire"]:
            continue
        n, a, d = int(r["n"]), r["area"], r["mind"]
        far = d is not None and d > 15
        near = d is not None and d <= 15
        if a >= 10000 and near:
            out["bignear"].append(n)
        if 1000 <= a < 10000:
            out["small"].append(n)
            if far:
                out["smallfar"].append(n)
        if a >= 1000 and far:
            out["far"].append(n)
        if 100 <= a < 1000:
            out["tiny"].append(n)
    return out


def load_blobs(path):
    d = np.loadtxt(path, delimiter=",", skiprows=1, dtype=np.float64)
    return d[:, 0], d[:, 1:11], d[:, 11].astype(int)


def report(name, score_of, neg, pos):
    s_neg = [score_of[n] for n in neg]
    s_pos = [score_of[n] for n in pos]
    b = pa.best_threshold(s_pos, s_neg)
    recall = sum(v >= b[0] for v in s_pos) / len(s_pos)
    fpr = sum(v >= b[0] for v in s_neg) / len(s_neg)
    print(f"  {name:<28} AUC {pa.auc(s_pos, s_neg):.3f}  bal.acc {b[1]:.3f} @ {b[0]:.4g}  (recall {recall:.2f}, false-pos {fpr:.2f})")
    return s_pos, s_neg


def main(argv):
    from sklearn.metrics import roc_auc_score
    from sklearn.tree import DecisionTreeClassifier, export_text

    n, X, truth = load_blobs(argv[1])
    rows = pa.load(argv[2])
    train, test = tp.temporal_split(n, 0.6)
    lab = truth >= 0
    print(f"blobs={len(n)} fire={int((truth == 1).sum())} non-fire={int((truth == 0).sum())} ambiguous={int((truth < 0).sum())}")
    tr, te = train & lab, test & lab
    print(f"train blobs {int(tr.sum())}  held-out blobs {int(te.sum())} (fire {int((truth[te] == 1).sum())})")
    print("blob-level AUC on held-out (fire vs non-fire blobs; <0.5 = fire has LOWER values):")
    for j, name in enumerate(FEATURES):
        print(f"  {name:<8}{roc_auc_score(truth[te], X[te, j]):7.3f}")

    cut = n[train].max()
    neg_all, pos_all = frame_sets(rows)
    neg = [f for f in neg_all if f > cut]
    pos = [f for f in pos_all if f > cut]
    print(f"\nheld-out frames: no-fire {len(neg)}, big+near fire {len(pos)} (lava frames NOT removed)")
    allf = neg + pos
    report("largest blob area (no shape)", frame_max(n, X[:, 0], allf), neg, pos)
    for depth in (2, 3, 4):
        clf = DecisionTreeClassifier(max_depth=depth, min_samples_leaf=20, class_weight="balanced", random_state=0)
        clf.fit(X[tr], truth[tr])
        p = clf.predict_proba(X)[:, 1]
        report(f"tree depth {depth} (any blob)", frame_max(n, p, allf), neg, pos)
        if depth == 3:
            print(export_text(clf, feature_names=FEATURES, decimals=3))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
