import json

import numpy as np
import pytest

import train_patch as tp


def eval_json(model, x):
    nodes = model["nodes"]
    i = 0
    while "leaf" not in nodes[i]:
        n = nodes[i]
        i = n["l"] if x[n["f"]] <= n["t"] else n["r"]
    return nodes[i]["leaf"]


def toy():
    rng = np.random.default_rng(1)
    X = rng.random((400, 12)).astype(np.float32)
    y = ((X[:, 0] > 0.6) & (X[:, 3] < 0.5)).astype(int)
    return X, y


def test_exported_tree_predicts_like_sklearn():
    from sklearn.tree import DecisionTreeClassifier

    X, y = toy()
    clf = DecisionTreeClassifier(max_depth=4, random_state=0).fit(X, y)
    model = tp.tree_to_json(clf)
    ours = np.array([eval_json(model, x) for x in X])
    assert np.allclose(ours, clf.predict_proba(X)[:, 1], atol=1e-6)


def test_exported_tree_is_preorder_with_children_after_parent():
    from sklearn.tree import DecisionTreeClassifier

    X, y = toy()
    model = tp.tree_to_json(DecisionTreeClassifier(max_depth=5, random_state=0).fit(X, y))
    for i, n in enumerate(model["nodes"]):
        if "leaf" not in n:
            assert n["l"] > i and n["r"] > i
            assert n["l"] < len(model["nodes"]) and n["r"] < len(model["nodes"])


def test_single_leaf_tree_exports():
    from sklearn.tree import DecisionTreeClassifier

    X = np.zeros((10, 12), dtype=np.float32)
    y = np.array([0, 1] * 5)
    model = tp.tree_to_json(DecisionTreeClassifier(max_depth=3).fit(X, y))
    assert model["nodes"] == [{"leaf": 0.5}]


def test_the_model_is_valid_json():
    from sklearn.tree import DecisionTreeClassifier

    X, y = toy()
    text = json.dumps(tp.tree_to_json(DecisionTreeClassifier(max_depth=3).fit(X, y)))
    assert json.loads(text)["nodes"]


def test_temporal_split_keeps_every_frame_whole_and_in_one_side():
    n = np.array([0, 0, 20, 20, 40, 60, 60, 80, 100, 100])
    train, test = tp.temporal_split(n, 0.6)
    assert not (train & test).any()
    assert (train | test).all()
    assert set(n[train]) & set(n[test]) == set()
    assert n[train].max() < n[test].min()
    # 60% of the time range 0..100 -> frames with n <= 60 train
    assert set(n[train]) == {0, 20, 40, 60}


def test_temporal_split_rejects_bad_fraction():
    with pytest.raises(ValueError):
        tp.temporal_split(np.array([1, 2, 3]), 0.0)
    with pytest.raises(ValueError):
        tp.temporal_split(np.array([1, 2, 3]), 1.0)
    with pytest.raises(ValueError):
        tp.temporal_split(np.array([]), 0.5)
