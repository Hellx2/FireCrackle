use fire_core::report::{format_row, LabelResult, Summary};
use fire_core::sync::SyncCode;

fn code(tick: u32, frame_id: u32) -> Option<SyncCode> {
    Some(SyncCode { tick, frame_id })
}

// ---------- one output row ----------

#[test]
fn a_matched_row_embeds_the_label_verbatim() {
    let label = "{\"f\":500,\"tick\":1000,\"f0\":3,\"f1\":19,\"fire\":false,\"fires\":[]}";
    let row = format_row(
        7,
        code(1000, 500),
        &LabelResult::Found {
            tick: 1000,
            line: label.to_string(),
        },
    );
    assert_eq!(
        row,
        format!("{{\"n\":7,\"tick\":1000,\"f\":500,\"tick_ok\":true,\"label\":{label}}}")
    );
}

#[test]
fn tick_ok_compares_modulo_24_bits() {
    let full = 0x1_000005u64; // the log keeps the full tick, the strip only the low 24 bits
    let row = format_row(
        0,
        code(5, 1),
        &LabelResult::Found {
            tick: full,
            line: "{\"f\":1,\"tick\":16777221}".to_string(),
        },
    );
    assert!(row.contains("\"tick_ok\":true"), "{row}");
}

#[test]
fn a_tick_mismatch_is_reported() {
    let row = format_row(
        0,
        code(1000, 500),
        &LabelResult::Found {
            tick: 1001,
            line: "{\"f\":500,\"tick\":1001}".to_string(),
        },
    );
    assert!(row.contains("\"tick_ok\":false"), "{row}");
    assert!(
        row.contains("\"label\":{\"f\":500"),
        "the label is still attached: {row}"
    );
}

#[test]
fn a_decoded_frame_without_a_label_row() {
    assert_eq!(
        format_row(3, code(10, 20), &LabelResult::Missing),
        "{\"n\":3,\"tick\":10,\"f\":20,\"tick_ok\":null,\"label\":null}"
    );
}

#[test]
fn an_undecodable_frame() {
    assert_eq!(
        format_row(5, None, &LabelResult::Missing),
        "{\"n\":5,\"tick\":null,\"f\":null,\"tick_ok\":null,\"label\":null}"
    );
}

#[test]
fn without_a_label_file_the_short_form_is_used() {
    assert_eq!(
        format_row(0, code(1000, 500), &LabelResult::NotRequested),
        "{\"n\":0,\"tick\":1000,\"f\":500}"
    );
    assert_eq!(
        format_row(1, None, &LabelResult::NotRequested),
        "{\"n\":1,\"tick\":null,\"f\":null}"
    );
}

// ---------- the summary line ----------

#[test]
fn summary_counts_everything_it_is_told_about() {
    let mut s = Summary::default();
    s.record(
        code(1, 1),
        &LabelResult::Found {
            tick: 1,
            line: String::new(),
        },
    ); // matched, ok
    s.record(
        code(1, 1),
        &LabelResult::Found {
            tick: 1,
            line: String::new(),
        },
    ); // repeat of the same id
    s.record(
        code(2, 2),
        &LabelResult::Found {
            tick: 9,
            line: String::new(),
        },
    ); // matched, tick mismatch
    s.record(None, &LabelResult::Missing); // undecodable
    s.record(code(3, 7), &LabelResult::Missing); // decoded, no label
    s.record(
        code(1, 1),
        &LabelResult::Found {
            tick: 1,
            line: String::new(),
        },
    ); // id went backwards (7 -> 1)
    assert_eq!(s.frames, 6);
    assert_eq!(s.decoded, 5);
    assert_eq!(s.undecodable, 1);
    assert_eq!(s.matched, 4);
    assert_eq!(s.unmatched, 1);
    assert_eq!(s.tick_mismatch, 1);
    assert_eq!(s.id_repeats, 1);
    assert_eq!(s.id_decreases, 1);
}

#[test]
fn undecodable_frames_do_not_reset_the_id_history() {
    let mut s = Summary::default();
    s.record(code(1, 10), &LabelResult::NotRequested);
    s.record(None, &LabelResult::NotRequested);
    s.record(None, &LabelResult::NotRequested);
    s.record(code(1, 11), &LabelResult::NotRequested); // 10 -> 11 across a gap is an increase, not a decrease
    assert_eq!(s.id_decreases, 0);
    assert_eq!(s.id_repeats, 0);
}

#[test]
fn without_labels_nothing_counts_as_matched_or_unmatched() {
    let mut s = Summary::default();
    s.record(code(1, 1), &LabelResult::NotRequested);
    assert_eq!((s.matched, s.unmatched), (0, 0));
}

#[test]
fn the_summary_line_lists_every_counter() {
    let mut s = Summary::default();
    s.record(
        code(1, 1),
        &LabelResult::Found {
            tick: 1,
            line: String::new(),
        },
    );
    let line = s.to_line();
    for key in [
        "frames=1",
        "decoded=1",
        "undecodable=0",
        "matched=1",
        "unmatched=0",
        "tick_mismatch=0",
        "id_repeats=0",
        "id_decreases=0",
    ] {
        assert!(line.contains(key), "{line} lacks {key}");
    }
}
