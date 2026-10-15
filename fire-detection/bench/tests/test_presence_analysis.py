import pytest

import presence_analysis as pa


def test_auc_perfect_separation():
    assert pa.auc([5, 6, 7], [1, 2, 3]) == 1.0


def test_auc_perfectly_wrong():
    assert pa.auc([1, 2, 3], [5, 6, 7]) == 0.0


def test_auc_identical_distributions_is_half():
    assert pa.auc([1, 2, 3, 4], [1, 2, 3, 4]) == pytest.approx(0.5)


def test_auc_ties_count_half():
    assert pa.auc([2], [2]) == 0.5
    assert pa.auc([2, 3], [2]) == pytest.approx(0.75)


def test_auc_known_value():
    # pos beats neg in 5 of 6 pairs
    assert pa.auc([3, 5], [1, 4, 2]) == pytest.approx(5 / 6)


def test_auc_empty_side_is_none():
    assert pa.auc([], [1]) is None
    assert pa.auc([1], []) is None


def test_best_threshold_balanced_accuracy():
    thr, bal = pa.best_threshold([10, 12, 15], [1, 2, 11])
    # thr=10 (everything >=10 is fire) and thr=12 both give balanced accuracy 5/6
    assert bal == pytest.approx((1.0 + 2 / 3) / 2)
    assert thr in (10, 12)


def test_best_threshold_perfect():
    thr, bal = pa.best_threshold([5, 6], [1, 2])
    assert bal == 1.0
    assert 2 < thr <= 5


def test_best_threshold_empty_side_is_none():
    assert pa.best_threshold([], [1]) is None


def test_load_splits_rows_and_ignores_blank_numbers(tmp_path):
    p = tmp_path / "f.csv"
    p.write_text("n,fire,area,mind,veil,r_cnt\n0,0,0.0,,0.000,5\n10,1,2000.0,12.5,0.000,50\n20,1,50.0,3.0,0.500,7\n")
    rows = pa.load(str(p))
    assert len(rows) == 3
    assert rows[0]["mind"] is None
    assert rows[1]["mind"] == 12.5
    assert rows[1]["r_cnt"] == 50.0


def test_groups_define_negatives_and_positives():
    rows = [
        {"fire": 0, "area": 0.0},
        {"fire": 1, "area": 50.0},
        {"fire": 1, "area": 2000.0},
        {"fire": 1, "area": 20000.0},
    ]
    neg, pos = pa.split(rows, min_area=1000)
    assert len(neg) == 1 and len(pos) == 2
    neg, pos = pa.split(rows, min_area=10000)
    assert len(pos) == 1
    # a tiny-but-present fire is neither a clean negative nor a clean positive
    assert all(r["area"] != 50.0 for r in neg + pos)
