use std::path::{Path, PathBuf};
use studio_core::autosave::*;
use studio_core::config::AutosaveConfig;
use studio_core::label::{log_line, Label, LabelError};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fc_autosave_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cfg() -> AutosaveConfig {
    AutosaveConfig::default()
}

fn fire(row: u8) -> Label {
    Label::Fire { sheet: 1, row, mirrored: false }
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

// ---------------------------------------------------------------- basics

#[test]
fn opening_a_missing_log_is_a_new_empty_one() {
    let dir = scratch("new");
    let (log, store, rep) = AutosaveLog::open(&dir.join("labels.jsonl"), 10, &cfg()).unwrap();
    assert_eq!(store.labelled_count(), 0);
    assert_eq!(store.frame_count(), 10);
    assert_eq!(log.lines_written(), 0);
    assert_eq!(rep, OpenReport { lines: 0, torn_tail_repaired: false });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn appended_lines_survive_a_reopen() {
    let dir = scratch("reopen");
    let path = dir.join("labels.jsonl");
    let (mut log, _, _) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    log.append(&[log_line(1, fire(5)), log_line(2, Label::Skip)]).unwrap();
    log.append(&[log_line(3, Label::NoFire)]).unwrap();
    assert_eq!(log.lines_written(), 3);
    drop(log);
    let (log2, store, rep) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert_eq!(store.get(1), fire(5));
    assert_eq!(store.get(2), Label::Skip);
    assert_eq!(store.get(3), Label::NoFire);
    assert_eq!((rep.lines, rep.torn_tail_repaired), (3, false));
    assert_eq!(log2.lines_written(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn appending_nothing_is_fine_and_the_last_write_wins() {
    let dir = scratch("lww");
    let path = dir.join("labels.jsonl");
    let (mut log, _, _) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    log.append(&[]).unwrap();
    log.append(&[log_line(1, fire(5)), log_line(1, fire(6)), log_line(2, Label::Skip), log_line(2, Label::Unset)]).unwrap();
    drop(log);
    let (_, store, _) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert_eq!(store.get(1), fire(6));
    assert_eq!(store.get(2), Label::Unset);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn blank_lines_and_crlf_are_tolerated() {
    let dir = scratch("crlf");
    let path = dir.join("labels.jsonl");
    std::fs::write(&path, format!("{}\r\n\r\n{}\r\n", log_line(0, Label::Skip), log_line(1, Label::NoFire))).unwrap();
    let (log, store, rep) = AutosaveLog::open(&path, 5, &cfg()).unwrap();
    assert_eq!(store.labelled_count(), 2);
    assert_eq!((rep.lines, rep.torn_tail_repaired), (2, false));
    assert_eq!(log.lines_written(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_fsync_setting_does_not_change_the_result() {
    for fsync in [true, false] {
        let dir = scratch(&format!("fsync{fsync}"));
        let path = dir.join("labels.jsonl");
        let (mut log, _, _) = AutosaveLog::open(&path, 10, &AutosaveConfig { fsync, ..cfg() }).unwrap();
        log.append(&[log_line(4, fire(9))]).unwrap();
        drop(log);
        assert_eq!(AutosaveLog::open(&path, 10, &cfg()).unwrap().1.get(4), fire(9));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn paths_with_spaces_and_non_ascii_work() {
    let dir = scratch("uni").join("my proj ü 火");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("labels file.jsonl");
    let (mut log, _, _) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    log.append(&[log_line(0, Label::Skip)]).unwrap();
    drop(log);
    assert_eq!(AutosaveLog::open(&path, 10, &cfg()).unwrap().1.labelled_count(), 1);
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

// ----------------------------------------------------------- crash repair

#[test]
fn a_torn_last_line_is_cut_off_the_file_before_anything_is_appended() {
    let dir = scratch("torn");
    let path = dir.join("labels.jsonl");
    let good = format!("{}\n{}\n", log_line(0, Label::Skip), log_line(1, fire(3)));
    std::fs::write(&path, format!("{good}{{\"frame\":2,\"label\":{{\"fire\":{{\"sheet\":1,\"ro")).unwrap();

    let (mut log, store, rep) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert!(rep.torn_tail_repaired);
    assert_eq!(rep.lines, 2);
    assert_eq!(store.labelled_count(), 2);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), good, "the torn tail is gone from the file itself");

    // the next append must not glue onto the torn line
    log.append(&[log_line(2, Label::NoFire)]).unwrap();
    drop(log);
    let (_, store2, rep2) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert!(!rep2.torn_tail_repaired);
    assert_eq!(store2.get(2), Label::NoFire);
    assert_eq!(store2.labelled_count(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newline_terminated_garbage_last_line_is_also_repaired() {
    let dir = scratch("torn2");
    let path = dir.join("labels.jsonl");
    let good = format!("{}\n", log_line(0, Label::Skip));
    std::fs::write(&path, format!("{good}{{\"frame\":1,\"la\n")).unwrap();
    let (_, store, rep) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert!(rep.torn_tail_repaired);
    assert_eq!(store.labelled_count(), 1);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), good);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_log_that_is_only_a_torn_line_becomes_empty() {
    let dir = scratch("torn3");
    let path = dir.join("labels.jsonl");
    std::fs::write(&path, "{\"frame\":0,\"lab").unwrap();
    let (log, store, rep) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    assert!(rep.torn_tail_repaired);
    assert_eq!((store.labelled_count(), log.lines_written()), (0, 0));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
    let _ = std::fs::remove_dir_all(&dir);
}

// --------------------------------------------------------- bad logs

#[test]
fn a_corrupt_middle_line_is_an_error_and_the_file_is_left_alone() {
    let dir = scratch("corrupt");
    let path = dir.join("labels.jsonl");
    let text = format!("{}\nnot json\n{}\n", log_line(0, Label::Skip), log_line(1, Label::Skip));
    std::fs::write(&path, &text).unwrap();
    match AutosaveLog::open(&path, 10, &cfg()) {
        Err(AutosaveError::Corrupt(LabelError::Parse { line, .. })) => assert_eq!(line, 2),
        other => panic!("{:?}", other.err()),
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "never rewrite a log we could not read");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_log_from_another_video_is_a_mismatch_not_a_silent_truncation() {
    let dir = scratch("mismatch");
    let path = dir.join("labels.jsonl");
    std::fs::write(&path, format!("{}\n", log_line(70, Label::Skip))).unwrap();
    match AutosaveLog::open(&path, 50, &cfg()) {
        Err(AutosaveError::VideoMismatch { frame: 70, count: 50 }) => {}
        other => panic!("{:?}", other.err()),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_invalid_label_in_the_log_is_corruption() {
    let dir = scratch("invalid");
    let path = dir.join("labels.jsonl");
    std::fs::write(&path, "{\"frame\":0,\"label\":{\"fire\":{\"sheet\":3,\"row\":0,\"mirrored\":false}}}\n").unwrap();
    assert!(matches!(AutosaveLog::open(&path, 10, &cfg()), Err(AutosaveError::Corrupt(LabelError::InvalidSheet(3)))));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn opening_a_directory_is_an_io_error() {
    let dir = scratch("isdir");
    assert!(matches!(AutosaveLog::open(&dir, 10, &cfg()), Err(AutosaveError::Io(_))));
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------- append failures

#[test]
fn a_failed_append_is_an_error_and_a_later_append_works_again() {
    let dir = scratch("fail");
    let sub = dir.join("proj");
    std::fs::create_dir_all(&sub).unwrap();
    let path = sub.join("labels.jsonl");
    let (mut log, _, _) = AutosaveLog::open(&path, 10, &cfg()).unwrap();
    log.append(&[log_line(0, Label::Skip)]).unwrap();
    std::fs::remove_dir_all(&sub).unwrap(); // the disk went away
    assert!(matches!(log.append(&[log_line(1, Label::Skip)]), Err(AutosaveError::Io(_))));
    assert_eq!(log.lines_written(), 1, "a failed append is not counted");
    std::fs::create_dir_all(&sub).unwrap(); // and came back
    log.append(&[log_line(1, Label::Skip)]).unwrap();
    assert_eq!(log.lines_written(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------ compaction

fn long_log(dir: &Path, cfg: &AutosaveConfig) -> (AutosaveLog, studio_core::label::LabelStore, PathBuf) {
    let path = dir.join("labels.jsonl");
    let (mut log, mut store, _) = AutosaveLog::open(&path, 10, cfg).unwrap();
    for row in 0..10u8 {
        store.set(1, fire(row)).unwrap();
        log.append(&[log_line(1, fire(row))]).unwrap();
    }
    store.set(2, Label::Skip).unwrap();
    log.append(&[log_line(2, Label::Skip)]).unwrap();
    store.set(3, Label::NoFire).unwrap();
    log.append(&[log_line(3, Label::NoFire)]).unwrap();
    store.clear(3).unwrap();
    log.append(&[log_line(3, Label::Unset)]).unwrap();
    (log, store, path)
}

#[test]
fn a_short_log_is_not_compacted() {
    let dir = scratch("nocompact");
    let (mut log, store, path) = long_log(&dir, &cfg());
    assert_eq!(log.lines_written(), 13);
    assert!(!log.compact_if_needed(&store).unwrap());
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 13);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_long_log_is_rewritten_compactly_with_the_same_labels() {
    let dir = scratch("compact");
    let c = AutosaveConfig { compact_after_lines: 5, ..cfg() };
    let (mut log, store, path) = long_log(&dir, &c);
    assert!(log.compact_if_needed(&store).unwrap());
    assert_eq!(log.lines_written(), 2, "frame 1 and frame 2; the cleared frame 3 is omitted");
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);
    assert_eq!(names(&dir), vec!["labels.jsonl".to_string()], "no temp file left behind");
    drop(log);
    let (_, reopened, _) = AutosaveLog::open(&path, 10, &c).unwrap();
    assert_eq!(reopened.iter(), store.iter());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn appends_after_a_compaction_go_to_the_new_file() {
    let dir = scratch("after");
    let c = AutosaveConfig { compact_after_lines: 5, ..cfg() };
    let (mut log, store, path) = long_log(&dir, &c);
    log.compact_if_needed(&store).unwrap();
    log.append(&[log_line(5, Label::Skip)]).unwrap();
    assert_eq!(log.lines_written(), 3);
    drop(log);
    let (_, reopened, _) = AutosaveLog::open(&path, 10, &c).unwrap();
    assert_eq!(reopened.get(5), Label::Skip);
    assert_eq!(reopened.get(1), fire(9));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compacting_an_empty_store_leaves_an_empty_file() {
    let dir = scratch("emptyc");
    let c = AutosaveConfig { compact_after_lines: 1, ..cfg() };
    let path = dir.join("labels.jsonl");
    let (mut log, mut store, _) = AutosaveLog::open(&path, 10, &c).unwrap();
    for i in 0..3u32 {
        store.set(i, Label::Skip).unwrap();
        log.append(&[log_line(i, Label::Skip)]).unwrap();
        store.clear(i).unwrap();
        log.append(&[log_line(i, Label::Unset)]).unwrap();
    }
    assert!(log.compact_if_needed(&store).unwrap());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
    assert_eq!(log.lines_written(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_stale_temp_file_from_a_crashed_compaction_does_not_break_the_next_one() {
    let dir = scratch("staletmp");
    let c = AutosaveConfig { compact_after_lines: 5, ..cfg() };
    let (mut log, store, path) = long_log(&dir, &c);
    for name in ["labels.jsonl.tmp", "labels.tmp", ".labels.jsonl.tmp"] {
        std::fs::write(dir.join(name), b"half a log").unwrap();
    }
    assert!(log.compact_if_needed(&store).unwrap());
    drop(log);
    assert_eq!(AutosaveLog::open(&path, 10, &c).unwrap().1.iter(), store.iter());
    let _ = std::fs::remove_dir_all(&dir);
}
