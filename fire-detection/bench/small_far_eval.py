"""Evaluate blob-texture detectors per difficulty class (small / far / tiny fire).

    python bench/small_far_eval.py NAME=blobs.csv,frames.csv [NAME=...]

Each config: depth-3 tree on blob features, trained on the first 60% of frames, scored on the held-out frames.
Negatives: every held-out frame with no fire (lava included). Positives per class, see blob_analysis.pos_sets.
"""
import sys

import numpy as np
from sklearn.tree import DecisionTreeClassifier

import blob_analysis as ba
import presence_analysis as pa
import train_patch as tp


def evaluate(name, blobs_path, frames_path):
    n, X, truth = ba.load_blobs(blobs_path)
    rows = pa.load(frames_path)
    train, _ = tp.temporal_split(n, 0.6)
    cut = n[train].max()
    lab = truth >= 0
    clf = DecisionTreeClassifier(max_depth=3, min_samples_leaf=20, class_weight="balanced", random_state=0)
    clf.fit(X[train & lab], truth[train & lab])
    p = clf.predict_proba(X)[:, 1]
    neg = [int(r["n"]) for r in rows if not r["fire"] and r["n"] > cut]
    sets = {k: [f for f in v if f > cut] for k, v in ba.pos_sets(rows).items()}
    allf = neg + [f for v in sets.values() for f in v]
    score = ba.frame_max(n, p, allf)
    # one operating point for all classes: the bignear-optimal threshold (a deployed detector has one threshold)
    ref = pa.best_threshold([score[f] for f in sets["bignear"]], [score[f] for f in neg])
    thr = ref[0]
    fpr = sum(score[f] >= thr for f in neg) / len(neg)
    print(f"\n== {name}: blobs={len(n)}  held-out no-fire frames={len(neg)}  threshold {thr:.3f} (best for big+near): false-pos {fpr:.2f}")
    print(f"  {'class':<10}{'frames':>7}{'AUC':>7}{'recall@thr':>12}{'best bal.acc':>14}")
    for k in ("bignear", "small", "far", "smallfar", "tiny"):
        pos = sets[k]
        if len(pos) < 5:
            print(f"  {k:<10}{len(pos):>7}   (too few frames)")
            continue
        sp = [score[f] for f in pos]
        sn = [score[f] for f in neg]
        rec = sum(v >= thr for v in sp) / len(sp)
        b = pa.best_threshold(sp, sn)
        print(f"  {k:<10}{len(pos):>7}{pa.auc(sp, sn):7.3f}{rec:12.2f}{b[1]:14.3f}")


def main(argv):
    for arg in argv[1:]:
        name, files = arg.split("=", 1)
        blobs, frames = files.split(",")
        evaluate(name, blobs, frames)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
