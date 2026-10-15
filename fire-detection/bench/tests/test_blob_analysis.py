import numpy as np
import pytest

import blob_analysis as ba


def test_frame_max_takes_the_best_blob_per_frame_and_zero_for_frames_without_blobs():
    out = ba.frame_max([10, 10, 20], [0.2, 0.9, 0.5], [10, 20, 30])
    assert out == {10: 0.9, 20: 0.5, 30: 0.0}


def test_frame_max_does_not_need_sorted_input():
    out = ba.frame_max([20, 10, 20, 10], [0.5, 0.1, 0.7, 0.3], [10, 20])
    assert out == {10: 0.3, 20: 0.7}


def test_frame_max_ignores_blobs_of_frames_not_asked_for():
    assert ba.frame_max([99], [1.0], [10]) == {10: 0.0}


def test_frame_max_empty_and_mismatch():
    assert ba.frame_max([], [], [1, 2]) == {1: 0.0, 2: 0.0}
    with pytest.raises(ValueError):
        ba.frame_max([1, 2], [0.5], [1])


def test_frame_sets_use_big_near_fire_and_no_fire_frames_only():
    rows = [
        {"n": 0.0, "fire": 0.0, "area": 0.0, "mind": None},
        {"n": 10.0, "fire": 1.0, "area": 20000.0, "mind": 5.0},
        {"n": 20.0, "fire": 1.0, "area": 20000.0, "mind": 30.0},  # far
        {"n": 30.0, "fire": 1.0, "area": 500.0, "mind": 3.0},  # small
        {"n": 40.0, "fire": 1.0, "area": 10000.0, "mind": 15.0},  # edges inclusive
    ]
    neg, pos = ba.frame_sets(rows)
    assert neg == [0]
    assert pos == [10, 40]


def test_pos_sets_partition_by_area_and_distance_with_exact_edges():
    rows = [
        {"n": 0.0, "fire": 1.0, "area": 20000.0, "mind": 5.0},   # bignear
        {"n": 1.0, "fire": 1.0, "area": 20000.0, "mind": 15.01}, # big but far
        {"n": 2.0, "fire": 1.0, "area": 1000.0, "mind": 15.0},   # small, near (edges inclusive)
        {"n": 3.0, "fire": 1.0, "area": 9999.9, "mind": 15.01},  # small, far
        {"n": 4.0, "fire": 1.0, "area": 999.9, "mind": 3.0},     # tiny
        {"n": 5.0, "fire": 1.0, "area": 100.0, "mind": 40.0},    # tiny, far
        {"n": 6.0, "fire": 1.0, "area": 99.9, "mind": 3.0},      # below everything
        {"n": 7.0, "fire": 0.0, "area": 0.0, "mind": None},      # no fire
        {"n": 8.0, "fire": 1.0, "area": 5000.0, "mind": None},   # no distance: only in 'small'
    ]
    s = ba.pos_sets(rows)
    assert s["bignear"] == [0]
    assert s["small"] == [2, 3, 8]
    assert s["far"] == [1, 3]
    assert s["smallfar"] == [3]
    assert s["tiny"] == [4, 5]
    assert set(s) == {"bignear", "small", "far", "smallfar", "tiny"}


def test_pos_sets_empty_input():
    s = ba.pos_sets([])
    assert all(v == [] for v in s.values())
