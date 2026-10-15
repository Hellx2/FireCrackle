use fire_core::video::parallel_frames;
use std::{
    io::Cursor,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
#[test]
fn preserves_every_frame_index_and_matches_serial_results() {
    let bytes: Vec<u8> = (0..90).collect();
    let mut out = Vec::new();
    parallel_frames(
        Cursor::new(bytes.clone()),
        3,
        30,
        4,
        &AtomicBool::new(false),
        |f| Ok(f.iter().map(|n| *n as u32).sum::<u32>()),
        |n, sum| out.push((n, sum)),
    )
    .unwrap();
    out.sort();
    assert_eq!(
        out,
        bytes
            .chunks(3)
            .enumerate()
            .map(|(n, f)| (n as u32, f.iter().map(|n| *n as u32).sum()))
            .collect::<Vec<_>>()
    );
}
#[test]
fn rejects_short_long_and_failed_processing_without_hanging() {
    for bytes in [vec![0; 8], vec![0; 10]] {
        assert!(parallel_frames(
            Cursor::new(bytes),
            3,
            3,
            4,
            &AtomicBool::new(false),
            |_| Ok(()),
            |_, _| {}
        )
        .is_err());
    }
    assert!(parallel_frames(
        Cursor::new(vec![0; 90]),
        3,
        30,
        4,
        &AtomicBool::new(false),
        |_| Err::<(), _>("bad frame".into()),
        |_, _| {}
    )
    .is_err());
}
#[test]
fn actually_runs_multiple_workers_with_a_bounded_concurrency() {
    let current = AtomicUsize::new(0);
    let maximum = AtomicUsize::new(0);
    parallel_frames(
        Cursor::new(vec![0; 60]),
        3,
        20,
        4,
        &AtomicBool::new(false),
        |_| {
            let active = current.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(active, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(2));
            current.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        },
        |_, _| {},
    )
    .unwrap();
    assert!(maximum.load(Ordering::SeqCst) > 1);
    assert!(maximum.load(Ordering::SeqCst) <= 4);
}
#[test]
fn cancellation_stops_without_processing_or_publishing_more_frames() {
    let cancel = AtomicBool::new(false);
    let mut out = Vec::new();
    parallel_frames(
        Cursor::new(vec![0; 300]),
        3,
        100,
        4,
        &cancel,
        |_| Ok(()),
        |n, value| {
            out.push((n, value));
            cancel.store(true, Ordering::Relaxed);
        },
    )
    .unwrap();
    assert_eq!(out.len(), 1);
}
