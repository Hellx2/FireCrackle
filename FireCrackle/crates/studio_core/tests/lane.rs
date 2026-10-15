use studio_core::label::{Label, LabelStore};
use studio_core::lane::*;
use studio_core::phase::{Flag, FlagKind, Severity};

fn fire() -> Label {
    Label::Fire {
        sheet: 1,
        row: 3,
        mirrored: false,
    }
}

fn flag(frame: u32, end: u32, severity: Severity) -> Flag {
    Flag {
        frame,
        end_frame: end,
        kind: FlagKind::PhaseExcursion,
        severity,
        phase_delta: 3.0,
    }
}

fn total(b: &[Bucket]) -> u32 {
    b.iter().map(|b| b.frames).sum()
}

// ------------------------------------------------------------- basics

#[test]
fn buckets_split_the_range_evenly_and_count_each_label_kind() {
    let mut s = LabelStore::new(100);
    s.fill_range(0, 4, fire()).unwrap();
    s.fill_range(5, 9, Label::NoFire).unwrap();
    s.fill_range(10, 12, Label::Skip).unwrap();
    let b = buckets(&s, &[], 0, 100, 10);
    assert_eq!(b.len(), 10);
    assert!(b.iter().all(|x| x.frames == 10));
    assert_eq!(
        (b[0].fire, b[0].no_fire, b[0].skip, b[0].unset),
        (5, 5, 0, 0)
    );
    assert_eq!(
        (b[1].fire, b[1].no_fire, b[1].skip, b[1].unset),
        (0, 0, 3, 7)
    );
    assert_eq!(b[5].unset, 10);
}

#[test]
fn uneven_ranges_distribute_the_remainder_and_lose_no_frame() {
    let s = LabelStore::new(10);
    let b = buckets(&s, &[], 0, 10, 3);
    assert_eq!(
        b.iter().map(|x| x.frames).collect::<Vec<_>>(),
        vec![3, 3, 4],
        "boundaries at floor(i*len/n)"
    );
    for n in 1..40 {
        assert_eq!(total(&buckets(&s, &[], 0, 10, n)), 10, "n = {n}");
    }
}

#[test]
fn more_buckets_than_frames_leaves_some_empty() {
    let s = LabelStore::new(3);
    let b = buckets(&s, &[], 0, 3, 6);
    assert_eq!(
        b.iter().map(|x| x.frames).collect::<Vec<_>>(),
        vec![0, 1, 0, 1, 0, 1]
    );
    assert_eq!(b[0], Bucket::default());
}

#[test]
fn a_sub_range_starts_at_its_own_start() {
    let mut s = LabelStore::new(100);
    s.fill_range(50, 54, fire()).unwrap();
    let b = buckets(&s, &[], 50, 60, 2);
    assert_eq!((b[0].frames, b[0].fire), (5, 5));
    assert_eq!((b[1].frames, b[1].fire, b[1].unset), (5, 0, 5));
}

#[test]
fn labels_outside_the_range_are_ignored() {
    let mut s = LabelStore::new(100);
    s.fill_range(0, 99, fire()).unwrap();
    let b = buckets(&s, &[], 40, 50, 1);
    assert_eq!((b[0].frames, b[0].fire), (10, 10));
}

// ---------------------------------------------------------- bad input

#[test]
fn degenerate_requests_give_no_buckets() {
    let s = LabelStore::new(100);
    assert!(buckets(&s, &[], 0, 100, 0).is_empty(), "no pixels");
    assert!(buckets(&s, &[], 50, 50, 5).is_empty(), "empty range");
    assert!(buckets(&s, &[], 60, 50, 5).is_empty(), "inverted range");
    assert!(
        buckets(&s, &[], 100, 200, 5).is_empty(),
        "range past the end"
    );
    assert!(
        buckets(&LabelStore::new(0), &[], 0, 10, 5).is_empty(),
        "empty store"
    );
}

#[test]
fn an_end_past_the_store_is_clamped() {
    let s = LabelStore::new(50);
    let b = buckets(&s, &[], 40, 1000, 2);
    assert_eq!(total(&b), 10);
}

// -------------------------------------------------------------- flags

#[test]
fn a_flag_marks_every_bucket_its_extent_overlaps() {
    let s = LabelStore::new(100);
    // an excursion over frames 20..=45 touches buckets 2, 3, 4 of ten
    let b = buckets(&s, &[flag(20, 45, Severity::Soft)], 0, 100, 10);
    let marked: Vec<usize> = (0..10).filter(|&i| b[i].flag.is_some()).collect();
    assert_eq!(marked, vec![2, 3, 4]);
}

#[test]
fn the_flag_extent_is_inclusive_at_both_ends() {
    let s = LabelStore::new(100);
    let b = buckets(&s, &[flag(9, 9, Severity::Soft)], 0, 100, 10);
    assert!(
        b[0].flag.is_some() && b[1].flag.is_none(),
        "frame 9 is the last frame of bucket 0"
    );
    let b = buckets(&s, &[flag(10, 10, Severity::Soft)], 0, 100, 10);
    assert!(b[0].flag.is_none() && b[1].flag.is_some());
}

#[test]
fn hard_beats_soft_in_a_shared_bucket() {
    let s = LabelStore::new(100);
    let b = buckets(
        &s,
        &[
            flag(1, 1, Severity::Soft),
            flag(2, 2, Severity::Hard),
            flag(3, 3, Severity::Soft),
        ],
        0,
        100,
        10,
    );
    assert_eq!(b[0].flag, Some(Severity::Hard));
    let b = buckets(&s, &[flag(1, 1, Severity::Soft)], 0, 100, 10);
    assert_eq!(b[0].flag, Some(Severity::Soft));
}

#[test]
fn flags_outside_the_range_are_ignored_and_partial_overlaps_count() {
    let s = LabelStore::new(100);
    let b = buckets(&s, &[flag(0, 30, Severity::Hard)], 50, 60, 2);
    assert!(b.iter().all(|x| x.flag.is_none()), "ends before the range");
    let b = buckets(&s, &[flag(40, 52, Severity::Hard)], 50, 60, 2);
    assert_eq!(
        b[0].flag,
        Some(Severity::Hard),
        "starts before the range, ends inside"
    );
    assert_eq!(b[1].flag, None);
    let b = buckets(&s, &[flag(58, 90, Severity::Soft)], 50, 60, 2);
    assert_eq!((b[0].flag, b[1].flag), (None, Some(Severity::Soft)));
}

#[test]
fn flags_do_not_change_the_counts() {
    let mut s = LabelStore::new(20);
    s.fill_range(0, 9, fire()).unwrap();
    let plain = buckets(&s, &[], 0, 20, 4);
    let flagged = buckets(&s, &[flag(2, 12, Severity::Hard)], 0, 20, 4);
    for (a, b) in plain.iter().zip(&flagged) {
        assert_eq!(
            (a.frames, a.unset, a.fire, a.no_fire, a.skip),
            (b.frames, b.unset, b.fire, b.no_fire, b.skip)
        );
    }
}

#[test]
fn a_big_store_is_summarised_quickly_and_exactly() {
    let mut s = LabelStore::new(650_000);
    s.fill_range(100_000, 300_000, fire()).unwrap();
    let b = buckets(&s, &[], 0, 650_000, 1500);
    assert_eq!(total(&b), 650_000);
    assert_eq!(b.iter().map(|x| x.fire).sum::<u32>(), 200_001);
}
