import json

import pytest

import coverage_report as cr


# ---------- builders for synthetic label rows (schema v3.1) ----------

def part(sheet, mirror, row, box=(100, 100, 200, 200), vis=1.0):
    return {"sheet": sheet, "mirror": mirror, "row": row, "box": list(box), "vis": vis}


def source(src, parts, dist=3.0, vis=1.0, box=(100, 100, 200, 200), **extra):
    s = {"src": src, "box": list(box), "vis": vis, "dist": dist, "parts": parts}
    s.update(extra)
    return s


def block(pos, parts, **kw):
    return source("block", parts, pos=list(pos), **kw)


def entity(eid, parts, **kw):
    return source("entity", parts, id=eid, type="minecraft:cow", **kw)


def overlay(parts, **kw):
    kw.setdefault("dist", 0.0)
    return source("overlay", parts, **kw)


def row(f, fires=(), w=1920, h=1080):
    return {"f": f, "tick": f // 3, "f0": 0, "f1": 0, "w": w, "h": h, "fire": bool(fires), "fires": list(fires)}


# ---------- basics ----------

def test_an_empty_input_does_not_crash():
    c = cr.coverage([])
    assert c["frames"] == 0 and c["fire_frames"] == 0 and c["fire_pct"] == 0.0
    assert isinstance(cr.format_report(c), str)


def test_counts_frames_and_fire_frames():
    rows = [row(0), row(1, [block((1, 2, 3), [part(0, False, 5)])]), row(2), row(3, [block((1, 2, 3), [part(0, False, 6)])])]
    c = cr.coverage(rows)
    assert c["frames"] == 4 and c["fire_frames"] == 2 and c["fire_pct"] == 50.0


def test_counts_sources_and_distinct_blocks_and_entities():
    rows = [
        row(0, [block((1, 2, 3), [part(0, False, 0)]), block((4, 5, 6), [part(1, False, 0)])]),
        row(1, [block((1, 2, 3), [part(0, False, 1)]), entity(7, [part(0, True, 0)]), overlay([part(1, True, 0)])]),
        row(2, [entity(7, [part(0, True, 1)]), entity(8, [part(0, True, 2)])]),
    ]
    c = cr.coverage(rows)
    assert c["entries"] == {"block": 3, "entity": 3, "overlay": 1}
    assert c["distinct_blocks"] == 2
    assert c["distinct_entities"] == 2


# ---------- distance, size, visibility buckets ----------

def test_distance_buckets_have_the_right_edges():
    fires = [block((i, 0, 0), [part(0, False, 0)], dist=d) for i, d in enumerate([0, 4.99, 5, 39.9, 40, 100])]
    c = cr.coverage([row(0, fires)])
    b = c["distance_buckets"]
    assert b["0-5"] == 2 and b["5-10"] == 1 and b["35-40"] == 1 and b["40+"] == 2


def test_box_size_is_relative_to_the_frame():
    tiny = block((0, 0, 0), [part(0, False, 0)], box=(0, 0, 20, 20))        # 400 px2 of 2.07M = 0.02%
    mid = block((1, 0, 0), [part(0, False, 0)], box=(0, 0, 100, 100))       # 0.48%
    big = block((2, 0, 0), [part(0, False, 0)], box=(0, 0, 500, 400))       # 9.6%
    huge = block((3, 0, 0), [part(0, False, 0)], box=(0, 0, 1000, 800))     # 38.6%
    c = cr.coverage([row(0, [tiny, mid, big, huge])])
    assert c["box_area"] == {"<0.1%": 1, "0.1-1%": 1, "1-10%": 1, ">=10%": 1}


def test_a_different_video_size_changes_the_relative_size():
    b = block((0, 0, 0), [part(0, False, 0)], box=(0, 0, 100, 100))
    small_frame = cr.coverage([row(0, [b], w=320, h=180)])
    big_frame = cr.coverage([row(0, [b], w=3840, h=2160)])
    assert small_frame["box_area"][">=10%"] == 1  # 10000 / 57600 = 17%
    assert big_frame["box_area"]["0.1-1%"] == 1  # 10000 / 8294400 = 0.12%


def test_visibility_buckets():
    fires = [block((i, 0, 0), [part(0, False, 0)], vis=v) for i, v in enumerate([0.05, 0.3, 0.7, 1.0])]
    c = cr.coverage([row(0, fires)])
    assert c["vis_buckets"] == {"<0.1": 1, "0.1-0.5": 1, "0.5-0.9": 1, ">=0.9": 1}


# ---------- texture row coverage ----------

def test_row_coverage_per_source_sheet_and_mirror():
    rows = [row(i, [block((0, 0, 0), [part(0, False, i % 32)])]) for i in range(64)]
    rows += [row(100 + i, [block((0, 0, 0), [part(1, True, i)])]) for i in range(10)]
    c = cr.coverage(rows)
    a = c["rows"][("block", 0, False)]
    assert a["rows_covered"] == 32 and a["entries"] == 64 and a["min_count"] == 2 and a["missing"] == []
    b = c["rows"][("block", 1, True)]
    assert b["rows_covered"] == 10 and b["missing"] == list(range(10, 32))


# ---------- gaps ----------

def rich_dataset():
    """Everything a good capture should contain."""
    rows = []
    f = 0
    for src in ("block", "entity"):
        for sheet in (0, 1):
            for mirror in (False, True):
                for rep in range(25):
                    for r in range(32):
                        d = (r * 2 + rep * 7) % 60
                        size = [(0, 0, 10, 10), (0, 0, 60, 60), (0, 0, 300, 300), (0, 0, 900, 700)][(r + rep) % 4]
                        vis = [0.05, 0.3, 0.7, 1.0][(r + rep) % 4]
                        p = [part(sheet, mirror, r, box=size, vis=vis)]
                        s = block((r, rep, 0), p, dist=d, vis=vis, box=size) if src == "block" else entity(r, p, dist=d, vis=vis, box=size)
                        rows.append(row(f, [s]))
                        f += 1
    for rep in range(25):
        for r in range(32):
            rows.append(row(f, [overlay([part(1, True, r)])]))
            f += 1
    rows += [row(f + i) for i in range(1500)]  # fire-free frames: about 12% of the whole set
    return rows


def test_a_rich_dataset_has_no_gaps():
    c = cr.coverage(rich_dataset())
    assert c["gaps"] == [], c["gaps"]


def test_a_sparse_dataset_lists_what_is_missing():
    rows = [row(i, [block((0, 0, 0), [part(0, False, 0)], dist=2.0)]) for i in range(50)]
    gaps = " | ".join(cr.coverage(rows)["gaps"]).lower()
    assert "fire-free" in gaps or "negative" in gaps
    assert "overlay" in gaps and "entity" in gaps
    assert "sheet 1" in gaps and "mirror" in gaps
    assert "farther" in gaps or "distance" in gaps
    assert "small" in gaps


def test_the_threshold_for_rare_rows_is_adjustable():
    rows = [row(i, [block((0, 0, 0), [part(0, False, i % 32)])]) for i in range(32 * 3)]
    strict = " | ".join(cr.coverage(rows, min_row_count=10)["gaps"])
    lenient = " | ".join(cr.coverage(rows, min_row_count=2)["gaps"])
    assert "block sheet 0 normal" in strict and "rows" in strict
    assert "block sheet 0 normal" not in lenient


# ---------- reading files ----------

def write(path, lines):
    path.write_text("\n".join(lines) + "\n", encoding="utf8")


def test_load_rows_reads_plain_label_logs_and_counts_junk(tmp_path):
    p = tmp_path / "labels.jsonl"
    write(p, [json.dumps(row(0)), "not json", json.dumps(row(1, [block((0, 0, 0), [part(0, False, 0)])])), ""])
    rows, stats = cr.load_rows(p)
    assert len(rows) == 2 and stats["bad_lines"] == 1 and stats["lines"] == 3


def test_load_rows_reads_the_decoders_joined_output(tmp_path):
    p = tmp_path / "joined.jsonl"
    good = {"n": 0, "tick": 0, "f": 0, "tick_ok": True, "label": row(0, [block((0, 0, 0), [part(0, False, 0)])])}
    unmatched = {"n": 1, "tick": 0, "f": 99, "tick_ok": None, "label": None}
    undecodable = {"n": 2, "tick": None, "f": None, "tick_ok": None, "label": None}
    write(p, [json.dumps(good), json.dumps(unmatched), json.dumps(undecodable)])
    rows, stats = cr.load_rows(p)
    assert len(rows) == 1 and stats["unlabelled"] == 2


def test_rows_from_the_older_schema_are_skipped_and_counted(tmp_path):
    old = {"f": 0, "tick": 0, "f0": 0, "f1": 0, "fire": True, "fires": [{"src": "block", "sheet": 0, "mirror": False, "row": 3}]}
    p = tmp_path / "old.jsonl"
    write(p, [json.dumps(old), json.dumps(row(1))])
    rows, stats = cr.load_rows(p)
    assert len(rows) == 1 and stats["old_schema"] == 1


def test_rows_without_the_framebuffer_size_are_counted_not_guessed(tmp_path):
    no_size = row(0)
    del no_size["w"], no_size["h"]
    p = tmp_path / "nosize.jsonl"
    write(p, [json.dumps(no_size), json.dumps(row(1))])
    rows, stats = cr.load_rows(p)
    assert len(rows) == 1 and stats["old_schema"] == 1


def test_the_command_line_prints_a_report(tmp_path, capsys):
    p = tmp_path / "labels.jsonl"
    write(p, [json.dumps(row(i, [block((0, 0, 0), [part(0, False, i % 32)])])) for i in range(64)])
    assert cr.main([str(p)]) == 0
    out = capsys.readouterr().out
    assert "frames" in out and "fire" in out and "gaps" in out.lower()


def test_the_command_line_reports_a_missing_file(tmp_path, capsys):
    assert cr.main([str(tmp_path / "nope.jsonl")]) == 1
    assert "error" in capsys.readouterr().err.lower()
    assert cr.main([]) == 2
