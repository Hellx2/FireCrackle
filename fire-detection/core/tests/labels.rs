use fire_core::labels::{parse_label_head, LabelIndex};
use std::path::PathBuf;

fn temp_file(name: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fire_core_labels_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, content).unwrap();
    p
}

// ---------- reading the id and tick off the start of a label row ----------

#[test]
fn reads_id_and_tick_from_the_row_start() {
    assert_eq!(
        parse_label_head("{\"f\":5,\"tick\":120,\"f0\":17,\"f1\":1,\"fire\":false,\"fires\":[]}"),
        Some((5, 120))
    );
    assert_eq!(parse_label_head("{\"f\":0,\"tick\":0}"), Some((0, 0)));
}

#[test]
fn handles_the_largest_values() {
    assert_eq!(
        parse_label_head("{\"f\":16777215,\"tick\":9876543210,\"f0\":3}"),
        Some((16777215, 9876543210))
    );
}

#[test]
fn anything_else_is_none() {
    assert_eq!(parse_label_head(""), None);
    assert_eq!(parse_label_head("not json"), None);
    assert_eq!(
        parse_label_head("{\"tick\":1,\"f\":2}"),
        None,
        "strict field order"
    );
    assert_eq!(parse_label_head("{\"f\":5}"), None, "no tick");
    assert_eq!(parse_label_head("{\"f\":-1,\"tick\":3}"), None);
    assert_eq!(
        parse_label_head("{\"f\":99999999999,\"tick\":3}"),
        None,
        "id does not fit 32 bits"
    );
    assert_eq!(parse_label_head("{\"f\":5,\"tick\":}"), None);
    assert_eq!(parse_label_head("{\"f\":,\"tick\":5}"), None);
}

// ---------- the offset index ----------

#[test]
fn indexes_and_fetches_rows_by_id() {
    let p = temp_file(
        "basic.jsonl",
        "{\"f\":1,\"tick\":10,\"f0\":0}\n\n{\"f\":2,\"tick\":10}\r\nnot json\n{\"f\":3,\"tick\":11}",
    );
    let mut idx = LabelIndex::build(&p).unwrap();
    assert_eq!(idx.rows, 3);
    assert_eq!(idx.bad_lines, 1, "blank lines are ignored, junk is counted");
    assert_eq!(idx.duplicate_ids, 0);
    assert_eq!(
        idx.get(1).unwrap(),
        Some((10, "{\"f\":1,\"tick\":10,\"f0\":0}".to_string()))
    );
    assert_eq!(
        idx.get(2).unwrap(),
        Some((10, "{\"f\":2,\"tick\":10}".to_string())),
        "CRLF is stripped"
    );
    assert_eq!(
        idx.get(3).unwrap(),
        Some((11, "{\"f\":3,\"tick\":11}".to_string())),
        "last line without newline"
    );
    assert_eq!(idx.get(99).unwrap(), None);
}

#[test]
fn random_access_works_in_any_order() {
    let p = temp_file(
        "order.jsonl",
        "{\"f\":1,\"tick\":1}\n{\"f\":2,\"tick\":2}\n{\"f\":3,\"tick\":3}\n",
    );
    let mut idx = LabelIndex::build(&p).unwrap();
    for id in [3u32, 1, 2, 2, 3, 1] {
        let (tick, line) = idx.get(id).unwrap().unwrap();
        assert_eq!(tick, id as u64);
        assert_eq!(line, format!("{{\"f\":{id},\"tick\":{id}}}"));
    }
}

#[test]
fn duplicate_ids_are_counted_and_the_first_wins() {
    let p = temp_file(
        "dup.jsonl",
        "{\"f\":5,\"tick\":1}\n{\"f\":6,\"tick\":2}\n{\"f\":5,\"tick\":99}\n",
    );
    let mut idx = LabelIndex::build(&p).unwrap();
    assert_eq!(idx.rows, 3);
    assert_eq!(idx.duplicate_ids, 1);
    assert_eq!(idx.get(5).unwrap().unwrap().0, 1);
}

#[test]
fn a_big_file_is_indexed_without_holding_the_rows() {
    let mut s = String::new();
    for i in 0..20_000u32 {
        s.push_str(&format!(
            "{{\"f\":{i},\"tick\":{},\"f0\":{},\"fire\":false,\"fires\":[]}}\n",
            i / 3,
            i % 32
        ));
    }
    let p = temp_file("big.jsonl", &s);
    let mut idx = LabelIndex::build(&p).unwrap();
    assert_eq!(idx.rows, 20_000);
    for id in [0u32, 19_999, 12_345, 7, 15_000] {
        let (tick, line) = idx.get(id).unwrap().unwrap();
        assert_eq!(tick, (id / 3) as u64);
        assert!(line.starts_with(&format!("{{\"f\":{id},")));
    }
}

#[test]
fn empty_and_missing_files() {
    let p = temp_file("empty.jsonl", "");
    let idx = LabelIndex::build(&p).unwrap();
    assert_eq!(idx.rows, 0);
    assert!(
        LabelIndex::build(&std::env::temp_dir().join("fire_core_no_such_labels_file.jsonl"))
            .is_err()
    );
}
