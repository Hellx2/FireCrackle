use fire_core::detect::{Candidate, Scored, Templates};
use fire_core::eval::{bucket_dist, bucket_size, bucket_veil, bucket_vis, judge, Stats};

const S0: &[u8] = include_bytes!("../testdata/fire_0.rgba");
const S1: &[u8] = include_bytes!("../testdata/fire_1.rgba");

fn c(sheet: u8, mirror: bool, row: u8) -> Candidate {
    Candidate { sheet, mirror, row }
}

fn ranked(list: &[Candidate]) -> Vec<Scored> {
    list.iter()
        .enumerate()
        .map(|(i, &cand)| Scored {
            cand,
            score: 1.0 - i as f64 * 0.1,
        })
        .collect()
}

#[test]
fn judge_exact_hit() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let o = judge(
        &t,
        c(0, false, 5),
        &ranked(&[c(0, false, 5), c(0, false, 6)]),
    );
    assert!(o.exact && o.top3 && o.group_ok && o.row_ok);
}

#[test]
fn judge_right_row_wrong_group_is_not_exact() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let o = judge(
        &t,
        c(0, false, 5),
        &ranked(&[c(1, false, 5), c(0, false, 5)]),
    );
    assert!(!o.exact && o.row_ok && !o.group_ok && o.top3);
}

#[test]
fn judge_right_group_wrong_row() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let o = judge(
        &t,
        c(1, true, 5),
        &ranked(&[c(1, true, 9), c(1, true, 10), c(1, true, 11), c(1, true, 5)]),
    );
    assert!(!o.exact && !o.top3 && o.group_ok && !o.row_ok);
}

#[test]
fn judge_top3_boundary() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let third = judge(
        &t,
        c(0, false, 1),
        &ranked(&[c(0, false, 7), c(0, false, 8), c(0, false, 1)]),
    );
    assert!(third.top3 && !third.exact);
    let fourth = judge(
        &t,
        c(0, false, 1),
        &ranked(&[
            c(0, false, 7),
            c(0, false, 8),
            c(0, false, 9),
            c(0, false, 1),
        ]),
    );
    assert!(!fourth.top3);
}

#[test]
fn judge_a_pixel_identical_candidate_counts_as_exact() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let cands = t.candidates();
    // find two distinct but pixel-identical candidates, if the textures have any
    for a in &cands {
        for b in &cands {
            if a != b && t.same_pixels(*a, *b) {
                assert!(judge(&t, *a, &ranked(&[*b])).exact);
                return;
            }
        }
    }
}

#[test]
fn judge_empty_ranking_is_all_false() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let o = judge(&t, c(0, false, 0), &[]);
    assert!(!o.exact && !o.top3 && !o.group_ok && !o.row_ok);
}

#[test]
fn buckets_have_exact_edges() {
    assert_eq!(bucket_vis(0.0), "vis <0.1");
    assert_eq!(bucket_vis(0.0999), "vis <0.1");
    assert_eq!(bucket_vis(0.1), "vis 0.1-0.5");
    assert_eq!(bucket_vis(0.5), "vis 0.5-0.9");
    assert_eq!(bucket_vis(0.9), "vis >=0.9");
    assert_eq!(bucket_vis(1.0), "vis >=0.9");
    assert_eq!(bucket_veil(0.0), "veil 0");
    assert_eq!(bucket_veil(0.001), "veil 0-0.25");
    assert_eq!(bucket_veil(0.25), "veil 0.25-0.5");
    assert_eq!(bucket_veil(0.5), "veil 0.5-0.75");
    assert_eq!(bucket_veil(0.75), "veil >=0.75");
    assert_eq!(bucket_dist(4.99), "dist 0-5");
    assert_eq!(bucket_dist(5.0), "dist 5-15");
    assert_eq!(bucket_dist(15.0), "dist 15-30");
    assert_eq!(bucket_dist(30.0), "dist 30-50");
    assert_eq!(bucket_dist(50.0), "dist 50+");
    assert_eq!(bucket_size(0.0), "box <10px");
    assert_eq!(bucket_size(9.9), "box <10px");
    assert_eq!(bucket_size(10.0), "box 10-20px");
    assert_eq!(bucket_size(20.0), "box 20-40px");
    assert_eq!(bucket_size(40.0), "box 40-80px");
    assert_eq!(bucket_size(80.0), "box >=80px");
}

#[test]
fn stats_count_per_key_and_overall() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    let mut s = Stats::default();
    let hit = judge(&t, c(0, false, 5), &ranked(&[c(0, false, 5)]));
    let miss = judge(
        &t,
        c(0, false, 5),
        &ranked(&[c(1, true, 9), c(0, false, 5)]),
    );
    s.record(&["a", "x"], hit);
    s.record(&["a", "y"], miss);
    s.record(&["b", "x"], hit);
    assert_eq!(s.total().n, 3);
    assert_eq!(s.total().exact, 2);
    assert_eq!(s.total().top3, 3);
    assert_eq!(s.get("a").unwrap().n, 2);
    assert_eq!(s.get("a").unwrap().exact, 1);
    assert_eq!(s.get("x").unwrap().n, 2);
    assert!(s.get("zzz").is_none());
}

#[test]
fn the_table_lists_every_key_with_percentages_and_survives_no_data() {
    let t = Templates::from_rgba(S0, S1).unwrap();
    assert!(Stats::default().table().contains("no parts"));
    let mut s = Stats::default();
    s.record(
        &["a"],
        judge(&t, c(0, false, 5), &ranked(&[c(0, false, 5)])),
    );
    let table = s.table();
    assert!(table.contains("a"));
    assert!(table.contains("100.0%"));
}
