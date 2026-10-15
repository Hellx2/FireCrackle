"""Train the patch classifier (decision tree) from fire-patches CSV. See core/DETECT_PLAN.md round 3.

    python bench/train_patch.py data/joined/patches20.csv --out data/joined/tree.json [--depth 8] [--neg-weight 20]

Trains on the first 60% of the clip (by frame number) and reports held-out numbers on the rest.
"""
import json
import sys

import numpy as np

FEATURES = ["pal", "strict", "bright", "white", "dark", "lum", "lumstd", "grad", "big", "sat", "warm", "ring"]


def tree_to_json(clf):
    """Preorder node list: split {"f","t","l","r"} (x[f] <= t goes left) or leaf {"leaf": P(fire)}."""
    t = clf.tree_
    nodes = []
    for i in range(t.node_count):
        if t.children_left[i] == -1:
            v = t.value[i][0]
            nodes.append({"leaf": float(v[1] / v.sum()) if v.sum() > 0 else 0.0})
        else:
            nodes.append({
                "f": int(t.feature[i]),
                "t": float(t.threshold[i]),
                "l": int(t.children_left[i]),
                "r": int(t.children_right[i]),
            })
    return {"nodes": nodes}


def temporal_split(n, frac):
    """Boolean masks (train, test): frames up to `frac` of the way through the frame-number range train."""
    n = np.asarray(n)
    if not 0.0 < frac < 1.0:
        raise ValueError(f"frac must be in (0, 1): {frac}")
    if n.size == 0:
        raise ValueError("no rows")
    cut = n.min() + frac * (n.max() - n.min())
    train = n <= cut
    return train, ~train


def load(path):
    data = np.loadtxt(path, delimiter=",", skiprows=1, dtype=np.float32)
    return data[:, 0], data[:, 3].astype(int), data[:, 4:]


def main(argv):
    from sklearn.metrics import roc_auc_score
    from sklearn.tree import DecisionTreeClassifier

    path = argv[1]
    out = argv[argv.index("--out") + 1] if "--out" in argv else None
    depth = int(argv[argv.index("--depth") + 1]) if "--depth" in argv else 8
    negw = float(argv[argv.index("--neg-weight") + 1]) if "--neg-weight" in argv else 20.0
    n, y, X = load(path)
    train, test = temporal_split(n, 0.6)
    print(f"rows={len(y)} train={train.sum()} test={test.sum()} positives train={y[train].sum()} test={y[test].sum()}")

    print("single-feature patch AUC on the held-out part (0.5 = useless):")
    for j, name in enumerate(FEATURES):
        print(f"  {name:<8}{roc_auc_score(y[test], X[test, j]):7.3f}")

    w = np.where(y == 1, 1.0, negw)
    clf = DecisionTreeClassifier(max_depth=depth, min_samples_leaf=200, random_state=0)
    clf.fit(X[train], y[train], sample_weight=w[train])
    p = clf.predict_proba(X[test])[:, 1]
    print(f"tree depth={depth} leaves={clf.get_n_leaves()}  held-out patch AUC = {roc_auc_score(y[test], p, sample_weight=w[test]):.3f}")
    for thr in (0.3, 0.5, 0.7):
        pred = p >= thr
        tp_ = float(w[test][pred & (y[test] == 1)].sum())
        fp = float(w[test][pred & (y[test] == 0)].sum())
        fn = float(w[test][~pred & (y[test] == 1)].sum())
        print(f"  thr {thr}: precision {tp_ / max(tp_ + fp, 1e-9):.3f}  recall {tp_ / max(tp_ + fn, 1e-9):.3f}  (negatives weighted x{negw:g})")
    imp = sorted(zip(clf.feature_importances_, FEATURES), reverse=True)
    print("importances:", ", ".join(f"{name} {v:.2f}" for v, name in imp if v > 0.01))
    if out:
        with open(out, "w") as f:
            json.dump(tree_to_json(clf), f)
        print("wrote", out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
