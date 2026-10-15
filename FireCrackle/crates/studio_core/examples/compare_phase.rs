//! Reproducible, paired phase-method benchmark. Ground-truth ticks are never detector inputs.
//! compare_phase JOINED VIDEO OUTPUT.json [TRIALS=2000] [SEED=1] [CONFIG.json]
use serde_json::{json, Value};
use std::{path::Path, time::Instant};
use studio_core::{
    config::{IndexConfig, PhaseResearchConfig, Settings},
    import::{probe, Tools},
    label::Label,
    phase::{self, FlagKind, FrameObs},
    phase_compare::{self as research, EventKind, Method},
};
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, l: usize, r: usize) -> usize {
        l + (self.next() % (r - l) as u64) as usize
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        return Err("compare_phase JOINED VIDEO OUTPUT [TRIALS] [SEED] [CONFIG.json]".into());
    }
    let trials = args
        .get(4)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(2000usize);
    let seed = args.get(5).map(|s| s.parse()).transpose()?.unwrap_or(1u64);
    let cfg: PhaseResearchConfig = if let Some(path) = args.get(6) {
        serde_json::from_str(&std::fs::read_to_string(path)?)?
    } else {
        Default::default()
    };
    let (_, index) = probe(
        &Tools::default(),
        Path::new(&args[2]),
        &IndexConfig::default(),
    )
    .map_err(|e| format!("probe: {e:?}"))?;
    let rows: Vec<Value> = std::fs::read_to_string(&args[1])?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let base: Vec<FrameObs> = rows
        .iter()
        .filter(|r| r["label"]["fire"].as_bool() == Some(true))
        .map(|r| {
            let n = r["n"].as_u64().unwrap() as u32;
            FrameObs {
                frame: n,
                pts: index.frames[n as usize].pts,
                label: Label::Fire {
                    sheet: 1,
                    row: r["label"]["f1"].as_u64().unwrap() as u8,
                    mirrored: false,
                },
            }
        })
        .collect();
    if base.len() < 130 {
        return Err("too few fire observations".into());
    }
    let mut rng = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) | 1);
    let mut cuts = Vec::new();
    let mut tried = [0usize; 5];
    let edges = [0.0, 1.0, 2.0, 4.0, 8.0, 16.01];
    for _ in 0..trials {
        let i = rng.range(60, base.len() - 60);
        let seconds = match rng.range(0, 3) {
            0 => 0.2 + rng.unit() * 2.0,
            1 => 2.0 + rng.unit() * 20.0,
            _ => 20.0 + rng.unit() * 90.0,
        };
        let j = base.partition_point(|o| o.pts < base[i].pts + seconds);
        if j >= base.len() - 5 {
            continue;
        }
        let removed = base[j].pts - base[i].pts;
        let shift = research::delta(-cfg.tick_rate * removed, 0.0).abs();
        let bucket = (0..5)
            .find(|&b| shift >= edges[b] && shift < edges[b + 1])
            .unwrap();
        tried[bucket] += 1;
        cuts.push((i, j, removed, bucket));
    }
    let mut reports = Vec::new();
    let legacy_cfg = phase::Config {
        tick_rate: cfg.tick_rate,
        ..Default::default()
    };
    let legacy = phase::analyze(&base, &legacy_cfg).unwrap();
    let mut settings = Settings::default();
    settings.phase = legacy_cfg;
    settings.phase_v2.bin_seconds = cfg.bin_seconds;
    settings.phase_v2.penalty_ticks2 = cfg.penalty_ticks2;
    settings.phase_v2.anchor_bins = cfg.clock_anchor_bins;
    settings.phase_v2.held_seconds = cfg.held_seconds;
    settings.phase_v2.held_max_gap_seconds = cfg.held_max_gap_seconds;
    settings.phase_v2.recovery_ticks = cfg.recovery_ticks;
    settings.phase_v2.supplement_min_ticks = cfg.min_change_ticks;
    let combined_clean = studio_core::phase_v2::analyze(&base, &settings).unwrap();
    for (method, is_combined) in [
        (None, false),
        (Some(Method::Fixed), false),
        (Some(Method::Pelt), false),
        (Some(Method::Pcid), false),
        (None, true),
    ] {
        let started = Instant::now();
        let clean = method.map(|m| research::compare(&base, m, &cfg).unwrap());
        let mut strict = [0usize; 5];
        let mut within_second = [0usize; 5];
        let mut raw_strict = [0usize; 5];
        let mut timings = Vec::new();
        for (trial, &(i, j, removed, bucket)) in cuts.iter().enumerate() {
            let mut cut = base[..i].to_vec();
            cut.extend(base[j..].iter().map(|o| FrameObs {
                pts: o.pts - removed,
                ..*o
            }));
            let near: Vec<u32> = base[j..].iter().take(4).map(|o| o.frame).collect();
            let before: Vec<u32> = base[i.saturating_sub(3)..i]
                .iter()
                .map(|o| o.frame)
                .collect();
            let time = base[j].pts;
            if let Some(m) = method {
                let result = research::compare(&cut, m, &cfg).unwrap();
                let baseline = clean.as_ref().unwrap();
                let new_boundary =
                    |f: u32| !baseline.boundaries.iter().any(|&b| b.abs_diff(f) <= 3);
                if result
                    .boundaries
                    .iter()
                    .any(|&f| new_boundary(f) && near.contains(&f))
                {
                    raw_strict[bucket] += 1;
                }
                let events: Vec<_> = result
                    .events
                    .iter()
                    .filter(|e| {
                        !baseline.events.iter().any(|b| {
                            b.first_frame == e.first_frame
                                && b.last_frame == e.last_frame
                                && b.kind == e.kind
                        })
                    })
                    .collect();
                let found = events.iter().any(|e| {
                    near.contains(&e.first_frame)
                        || (e.kind == EventKind::RecoveredDeparture
                            && before.contains(&e.last_frame))
                });
                if found {
                    strict[bucket] += 1;
                }
                let distance = events
                    .iter()
                    .flat_map(|e| [e.first_frame, e.last_frame])
                    .filter_map(|f| {
                        base.binary_search_by_key(&f, |o| o.frame)
                            .ok()
                            .map(|k| (base[k].pts - time).abs())
                    })
                    .fold(f64::INFINITY, f64::min);
                if found || distance <= 1.0 {
                    within_second[bucket] += 1;
                    timings.push(distance);
                }
            } else {
                let result = if is_combined {
                    studio_core::phase_v2::analyze(&cut, &settings).unwrap()
                } else {
                    phase::analyze(&cut, &legacy_cfg).unwrap()
                };
                let baseline = if is_combined {
                    &combined_clean
                } else {
                    &legacy
                };
                let new = |f: &phase::Flag| {
                    !baseline
                        .flags
                        .iter()
                        .any(|g| g.frame == f.frame && g.end_frame == f.end_frame)
                };
                let found = result.flags.iter().any(|f| {
                    new(f)
                        && (near.contains(&f.frame)
                            || (f.kind == FlagKind::PhaseExcursion
                                && before.contains(&f.end_frame)))
                });
                if found {
                    strict[bucket] += 1;
                }
                let distance = result
                    .flags
                    .iter()
                    .filter(|f| new(f))
                    .flat_map(|f| [f.frame, f.end_frame])
                    .filter_map(|f| {
                        base.binary_search_by_key(&f, |o| o.frame)
                            .ok()
                            .map(|k| (base[k].pts - time).abs())
                    })
                    .fold(f64::INFINITY, f64::min);
                if found || distance <= 1.0 {
                    within_second[bucket] += 1;
                }
            }
            if trial % 250 == 0 {
                eprintln!("{method:?}: {trial}/{}", cuts.len());
            }
        }
        timings.sort_by(f64::total_cmp);
        let clean_json = if is_combined {
            json!({"flags":combined_clean.flags,"runs":combined_clean.runs.len(),"flag_details":combined_clean.flag_details,"lag_intervals":combined_clean.lag_intervals,"reference_phase":combined_clean.reference_phase,"clock_residuals":combined_clean.clock_residuals})
        } else if let Some(a) = &clean {
            json!({"events":a.events,"boundaries":a.boundaries,"bins":a.bins,"reference_phase":a.reference_phase,"objective":a.objective,"clock_residuals":a.clock_residuals,
            "held_events":a.events.iter().filter(|e|e.kind==EventKind::HeldAnimation).count(),"recovered_events":a.events.iter().filter(|e|e.kind==EventKind::RecoveredDeparture).count()})
        } else {
            json!({"flags":legacy.flags,"runs":legacy.runs.len()})
        };
        let gap_check = if is_combined {
            let masked: Vec<_> = base
                .iter()
                .copied()
                .filter(|o| o.pts < 20.0 || o.pts >= 120.0)
                .collect();
            let first_after = masked.iter().find(|o| o.pts >= 120.0).unwrap().frame;
            let shifted: Vec<_> = masked
                .iter()
                .copied()
                .map(|mut o| {
                    if o.pts >= 120.0 {
                        if let Label::Fire {
                            sheet,
                            row,
                            mirrored,
                        } = o.label
                        {
                            o.label = Label::Fire {
                                sheet,
                                row: (row + 5) % 32,
                                mirrored,
                            };
                        }
                    }
                    o
                })
                .collect();
            let matching = studio_core::phase_v2::analyze(&masked, &settings).unwrap();
            let changed = studio_core::phase_v2::analyze(&shifted, &settings).unwrap();
            let near =
                |a: &phase::Analysis| a.flags.iter().any(|f| f.frame.abs_diff(first_after) <= 3);
            json!({"masked_seconds":[20.0,120.0],"first_observation_after":first_after,"matching_gap_flag":near(&matching),"five_tick_mismatch_flag":near(&changed),"matching_reference":matching.reference_phase,"shifted_reference":changed.reference_phase})
        } else if let Some(m) = method {
            let start = 20.0;
            let end = 120.0;
            let masked: Vec<_> = base
                .iter()
                .copied()
                .filter(|o| o.pts < start || o.pts >= end)
                .collect();
            let first_after = masked.iter().find(|o| o.pts >= end).unwrap().frame;
            let shifted: Vec<_> = masked
                .iter()
                .copied()
                .map(|mut o| {
                    if o.pts >= end {
                        if let Label::Fire {
                            sheet,
                            row,
                            mirrored,
                        } = o.label
                        {
                            o.label = Label::Fire {
                                sheet,
                                row: (row + 5) % 32,
                                mirrored,
                            };
                        }
                    }
                    o
                })
                .collect();
            let matching = research::compare(&masked, m, &cfg).unwrap();
            let changed = research::compare(&shifted, m, &cfg).unwrap();
            let near = |a: &research::ResultSet| {
                a.boundaries.iter().any(|&f| f.abs_diff(first_after) <= 3)
            };
            json!({"masked_seconds":[start,end],"first_observation_after":first_after,"matching_gap_boundary":near(&matching),"five_tick_mismatch_boundary":near(&changed),"matching_reference":matching.reference_phase,"shifted_reference":changed.reference_phase})
        } else {
            Value::Null
        };
        let report = json!({"method":if is_combined {"Combined".into()}else{method.map(|m|format!("{m:?}")).unwrap_or("Legacy".into())},"clean":clean_json,"tried":tried,"caught_strict_4_observations":strict,"raw_boundary_strict_4_observations":raw_strict,"caught_within_1_second":within_second,"seconds":started.elapsed().as_secs_f64(),"median_localisation_seconds":timings.get(timings.len()/2),"long_gap_check":gap_check});
        println!(
            "{}",
            json!({"method":report["method"],"strict":strict,"within_second":within_second,"tried":tried,"seconds":report["seconds"]})
        );
        reports.push(report);
        // Save after every method so an interrupted run still has explicit partial evidence.
        let output = json!({"joined":args[1],"video":args[2],"trials_requested":trials,"valid_cuts":cuts.len(),"seed":seed,"config":cfg,"combined_settings":settings,"fire_observations":base.len(),"reports":reports});
        std::fs::write(&args[3], serde_json::to_string_pretty(&output)?)?;
    }
    let diagnostic = research::clock_diagnostic(&base, &cfg).unwrap();
    let mut output: Value = serde_json::from_str(&std::fs::read_to_string(&args[3])?)?;
    output["clock_diagnostic"] = json!(diagnostic);
    std::fs::write(&args[3], serde_json::to_string_pretty(&output)?)?;
    Ok(())
}
