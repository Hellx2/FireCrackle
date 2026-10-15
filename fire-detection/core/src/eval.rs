//! Scoring of the baseline against labels. See DETECT_PLAN.md.

use crate::detect::{Candidate, Scored, Templates};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct Outcome {
    /// Best candidate is the truth (or pixel-identical to it).
    pub exact: bool,
    /// Truth (or a pixel-identical one) is among the three best.
    pub top3: bool,
    /// Best candidate has the right sheet and mirror (row ignored).
    pub group_ok: bool,
    /// Best candidate has the right row (sheet and mirror ignored).
    pub row_ok: bool,
}

pub fn judge(t: &Templates, truth: Candidate, ranked: &[Scored]) -> Outcome {
    let hit = |c: Candidate| c == truth || t.same_pixels(c, truth);
    let Some(best) = ranked.first() else {
        return Outcome::default();
    };
    Outcome {
        exact: hit(best.cand),
        top3: ranked.iter().take(3).any(|s| hit(s.cand)),
        group_ok: best.cand.sheet == truth.sheet && best.cand.mirror == truth.mirror,
        row_ok: best.cand.row == truth.row,
    }
}

pub fn bucket_vis(v: f64) -> &'static str {
    if v < 0.1 {
        "vis <0.1"
    } else if v < 0.5 {
        "vis 0.1-0.5"
    } else if v < 0.9 {
        "vis 0.5-0.9"
    } else {
        "vis >=0.9"
    }
}

pub fn bucket_veil(v: f64) -> &'static str {
    if v <= 0.0 {
        "veil 0"
    } else if v < 0.25 {
        "veil 0-0.25"
    } else if v < 0.5 {
        "veil 0.25-0.5"
    } else if v < 0.75 {
        "veil 0.5-0.75"
    } else {
        "veil >=0.75"
    }
}

pub fn bucket_dist(v: f64) -> &'static str {
    if v < 5.0 {
        "dist 0-5"
    } else if v < 15.0 {
        "dist 5-15"
    } else if v < 30.0 {
        "dist 15-30"
    } else if v < 50.0 {
        "dist 30-50"
    } else {
        "dist 50+"
    }
}

/// `v`: the longer side of the box in video pixels.
pub fn bucket_size(v: f64) -> &'static str {
    if v < 10.0 {
        "box <10px"
    } else if v < 20.0 {
        "box 10-20px"
    } else if v < 40.0 {
        "box 20-40px"
    } else if v < 80.0 {
        "box 40-80px"
    } else {
        "box >=80px"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    pub n: u64,
    pub exact: u64,
    pub top3: u64,
    pub group_ok: u64,
    pub row_ok: u64,
}

impl Tally {
    fn add(&mut self, o: Outcome) {
        self.n += 1;
        self.exact += o.exact as u64;
        self.top3 += o.top3 as u64;
        self.group_ok += o.group_ok as u64;
        self.row_ok += o.row_ok as u64;
    }
}

#[derive(Default)]
pub struct Stats {
    keys: BTreeMap<String, Tally>,
    total: Tally,
}

impl Stats {
    /// Counts one judged part under every key (source, bucket, ...) and in the total.
    pub fn record(&mut self, keys: &[&str], o: Outcome) {
        self.total.add(o);
        for k in keys {
            self.keys.entry((*k).to_string()).or_default().add(o);
        }
    }

    pub fn total(&self) -> Tally {
        self.total
    }

    pub fn get(&self, key: &str) -> Option<Tally> {
        self.keys.get(key).copied()
    }

    pub fn table(&self) -> String {
        if self.total.n == 0 {
            return "no parts evaluated\n".to_string();
        }
        let pct = |a: u64, n: u64| format!("{:5.1}%", 100.0 * a as f64 / n as f64);
        let mut out = format!(
            "{:<16} {:>9} {:>7} {:>7} {:>9} {:>7}\n",
            "", "parts", "exact", "top3", "sheet+mir", "row"
        );
        let mut line = |name: &str, t: &Tally| {
            out.push_str(&format!(
                "{:<16} {:>9} {:>7} {:>7} {:>9} {:>7}\n",
                name,
                t.n,
                pct(t.exact, t.n),
                pct(t.top3, t.n),
                pct(t.group_ok, t.n),
                pct(t.row_ok, t.n)
            ));
        };
        line("ALL", &self.total);
        for (k, t) in &self.keys {
            line(k, t);
        }
        out
    }
}
