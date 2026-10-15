# First-person fire overlay: acceptance status

2026-10-15. Priority: complete vanilla 1.16.5 first-person overlay detection before a different AI approach. The existing silhouette/chromaticity matcher remains the baseline. This pass adds a video-only entry point and reproducible acceptance reporting; it does not retune the matcher.

## What works now

`core/src/bin/fire_pov.rs` accepts a video without a mod, strip decoder, joined labels, or a phase clock. It tries the existing 70/60 degree overlay geometries, reverses a detected GUI veil, and uses colour evidence to select the geometry. At the fixed default threshold of 0.5 it emits JSONL with a texture `row` only when accepted; otherwise `row` is null. `candidate_row` is diagnostic and must not be treated as a detected texture. Evidence is a matching score, not a calibrated probability. Below-threshold output does not establish that fire is absent.

From `fire-detection/` (ffmpeg and ffprobe must be on PATH; explicit paths can be passed with `--ffmpeg` and `--ffprobe`):

```powershell
cargo build --release --manifest-path core/Cargo.toml --bin fire_pov
& .\core\target\release\fire_pov.exe '.\data\video\2026-10-15 18-18-16.mkv' --stride 5
```

The default asset directory resolves from the source tree; use `--assets DIR` containing `fire_1.rgba` when moving the executable. `--limit N` stops after N emitted predictions. Frame `n` is the decoded frame index, not a timestamp; stride defaults to 1. Input must show the complete, uncropped game viewport. Crops, letterboxing, arbitrary HUD/resource packs and other Minecraft versions are not established as supported. Sheet is always fire_1; this command does not detect world fire blocks.

## Measured baseline

Report from `overlay5_clip1_h.csv` and `overlay5_clip2_h.csv`, every fifth frame, fixed `fin_ev >= 0.5`. These are existing evaluation outputs, not unseen recordings. The matcher constants were already selected using both recordings.

| Metric | Clip 1 | Clip 2 |
|---|---:|---:|
| Sampled frames | 6526 | 3911 |
| Labelled overlay / no overlay | 194 / 6332 | 932 / 2979 |
| Correct candidate texture row, irrespective of acceptance | 194/194 (100%) | 928/932 (99.57%) |
| Accepted presence AND correct row | 174/194 (89.69%) | 895/932 (96.03%) |
| False positives | 0/6332 | 0/2979 |
| Below-threshold overlay frames | 20 | 37 |
| Presence AND correct row, visibility < 0.15 | 7/22 | 90/114 |
| Presence AND correct row, visibility 0.15 to < 0.3 | 129/134 | 508/521 |
| Presence AND correct row, visibility >= 0.3 | 38/38 | 297/297 |
| Presence AND correct row, label veil >= 0.6 | no samples | 114/135 |

All accepted overlay rows in these samples are correct. Zero observed false positives on these correlated samples does not imply a zero population error rate. Groups use truth visibility/veil only for scoring; predictions never read them. Metrics are per frame, not independent events or full-frame-rate validation.

The four wrong candidate rows are all rejected by the threshold:

| Clip 2 frame | Truth -> candidate | Circular offset | Visibility | Evidence |
|---|---|---:|---:|---:|
| 9745 | 12 -> 11 | -1 | 0.096 | 0.2433 |
| 14350 | 13 -> 11 | -2 | 0.092 | 0.0345 |
| 14355 | 14 -> 11 | -3 | 0.140 | 0.0592 |
| 15025 | 13 -> 11 | -2 | 0.044 | 0.0265 |

Presence is the larger remaining limitation than candidate row accuracy. Lowering the threshold without measuring false positives and wrong accepted rows would hide that limitation. Temporal filling is also unsafe without separate validation of entry/exit, frame drops, pauses and cuts.

## Reproduce the report

```powershell
python bench/overlay_analysis.py data/joined/overlay5_clip1_h.csv data/joined/overlay5_clip2_h.csv
python -m pytest bench/tests/test_overlay_analysis.py
cargo test --manifest-path core/Cargo.toml --bin fire_pov --test silhouette --test veil
```

The report is JSON on stdout. It includes per-clip confusion counts, candidate-row accuracy, accepted-and-correct accuracy, visibility/veil groups, all false-negative/false-positive frame indices and circular row errors. Undefined rates are null. It rejects duplicate indices, invalid rows and nonfinite values. `--threshold` is explicit and fixed; no threshold is fitted to an evaluation clip.

Input SHA-256:

- `overlay5_clip1_h.csv`: `9C1A155B30DEF47E3CD48B9EDB519AE439E7D180F6B0382DBF6CD42C085B8121`
- `overlay5_clip2_h.csv`: `B031ABFF438B26FC4753900B363584DD3EA361E0AC06AD2EDCE6A7D9525635C3`

Validation this pass: nine Python reporting tests and three Rust acceptance tests written first and observed failing before implementation; unchanged assertions pass. Existing 22 silhouette tests and 16 veil tests pass. Release command builds. A video-only smoke check decoded 2996 real clip1 frames and compared 600 sampled predictions to the existing CSV: all candidate rows/evidence agree; 49 accepted predictions. This checks the no-label entry point independently of label parsing.

Full clip2 check: the new command decoded all 19,553 frames and emitted 3,911 stride-five predictions. Every candidate row, evidence score (within CSV rounding), FOV choice and veil decision matched `overlay5_clip2_h.csv`; 895 predictions were accepted. Help and six invalid-argument cases were also checked. These are parity checks against the existing evaluation, not a new accuracy holdout.

The video-only command uses the shared `SilhouetteResult.colour_evidence` field. The matcher and its performance changes are part of the source baseline; this report records the earlier acceptance measurements separately from later performance work. Local evaluation CSVs and original recordings are intentionally excluded from source control; the hashes above identify the inputs needed to reproduce these recording-specific numbers.

## Gates before calling first-person detection finished

1. Improve low-visibility presence evidence, keeping abstention and checking every new accepted row against labels. Investigate the 57 below-threshold frames before changing thresholds. The four wrong candidates above need occlusion-aware evidence or explicit ambiguity, not forced output.
2. Freeze constants and test on a new recording: ordinary scenes plus lava, inventory/pause, held items, other GUI scales and viewport aspect ratios. Declare the supported visibility range and acceptable recall/false-positive rates before inspecting that recording.
3. Apply the existing compression/degradation tools to that held-out recording and repeat the same report. The original recordings do not establish YouTube robustness.
4. Measure full-frame-rate runtime and entry/exit behavior; stride-five success is not a realtime performance result. Validate any temporal smoothing separately against freezes, missing frames and cuts.

The first-person milestone remains open. No CNN or other learned detector was introduced.
