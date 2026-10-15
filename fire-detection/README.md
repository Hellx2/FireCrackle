# Fire detection tools

Goal: given a video frame, report which fire texture is on screen, as `{frame} {sheet} {normal|mirrored}`,
robustly after YouTube-style compression.

## Decisions

- Mod: **Fabric, Minecraft 1.16.5** for the first version (test ground). Built with the Gradle wrapper; JDK 17 is installed.
- Performance-critical backend (sync decode, detector) is **Rust**; OBS control is Python (control plane only).
## Status (what exists and how far it is verified)

| Part | What | Verified |
|---|---|---|
| `mod/` | Fabric 1.16.5 mod: sync strip (tick + frame id) on every frame, per-frame label log (`fires[]`: one box per fire block / burning entity / on-fire overlay, with sheet, mirror, row parts) | 152 unit tests; run in a real dev client (`gradlew runClient`): log complete, boxes checked by eye |
| `core/` | Rust: strip codec, `fire-decode` (video -> tick/frame id -> label row), `stamp-test-video` | 66 tests incl. real x264 round trips; ~2,260 fps (37x realtime) on 1080p60 |
| `obs/` | Python OBS start/stop controller | 9 tests with a fake client; never run against a real OBS |
| `augment/` | YouTube-like degradation chains, calibration tool, strip robustness matrix | 49 tests with real ffmpeg; research in `augment/YOUTUBE_ENCODING.md` |
| detector | not started | |

Not yet verified end to end: the strip decoding from a **real OBS recording of the mod** (only synthetic stamped
video so far), and the YouTube-like ranges against **real YouTube-delivered files** (they are assumptions until
calibrated, see `augment/YOUTUBE_ENCODING.md`).

Run things (Windows; Gradle must be run from your own terminal, see `NOTES-minecraft-1.16.5.md`):

- Mod: `cd mod`, `.\gradlew.bat build` (jar: `mod/build/libs/firelabel-0.1.0.jar`), `.\gradlew.bat runClient`
  (dev client with the debug overlay). Labels: `<game dir>/firelabel/labels-*.jsonl`.
- Rust: `cd core`, `cargo build --release`, then
  `target\release\fire-decode VIDEO --labels labels.jsonl --out joined.jsonl`.
- Python tools: `.venv\Scripts\python augment\robustness.py`, `augment\degrade.py`, `augment\calibrate.py`.

## Pipeline

1. **Mod (ground truth).** Per rendered frame: the strip (tick + frame id) is drawn into the frame and a label
   row is written: `fire`, and per fire source one box with its texture variants. Schema: `mod/PLAN.md`.
2. **Recorder.** OBS records the game (`obs/` can start and stop it). The strip lets labels be joined to video
   frames even after re-encoding, dropped or duplicated frames. Do not rely on clock sync.
3. **Join.** `fire-decode` reads the strip of every video frame and attaches the label row, flagging tick
   mismatches (wrong log / splice / id wrap), duplicated frames and dropped or spliced footage.
4. **Degrade.** `augment/` re-encodes with YouTube-like chains (recorder x264 -> H.264 / VP9 / AV1 at a ladder
   rung), recorded in manifests; calibrate it with real YouTube downloads of your own test upload.
5. **Detector.** Baseline first, ML only if the baseline fails (see below).
6. **Benchmark.** Accuracy per (compression level, distance, bbox size, frame); confusion matrix to see
   which frames get mistaken for each other.

## Detector plan

- **Baseline: classical template matching.** Crop the fire region, normalise (grayscale + local contrast),
  match against all 32 x 2 x {normal, mirrored} templates (NCC). Fast, explainable, no training.
- **If it fails after compression** (small/far fire, perspective, color banding): small CNN classifier
  trained on mod-labelled crops, with compression augmentation. Plain color matching alone is unlikely
  to be enough: adjacent frames differ mostly in flame shape, and chroma subsampling destroys color detail.
- The mod labels are useful either way, as training data or as the benchmark for the CV approach.

## Risks / open questions

- Fire is rendered on crossed planes with perspective, so crops are not axis-aligned sprites.
- Video 60fps vs 20 TPS: each texture lasts ~3 video frames; the sync code must disambiguate.
- Mirrored vs normal may be indistinguishable on near-symmetric frames; report ambiguity, not a guess.
- Frame count / sheet layout (32 frames per sheet?) needs confirming from the actual assets.
- Is the texture frame purely a function of tick (the phase formula assumes so)? The mod can verify this.
- Phase formula: the example table matches `(tick - texture) % 32`, not `(texture + tick) % 32`.

## Layout (planned)

- `mod/`       Minecraft mod (Fabric 1.16.5)
- `core/`      Rust: strip codec, `fire-decode`, `stamp-test-video` (later: the detector)
- `obs/`       OBS controller (Python)
- `augment/`   YouTube-like degradation, calibration, robustness matrix
- `sync/`      strip format and decoder plan; `NOTES-minecraft-1.16.5.md` game internals
- later: `data/` (recordings, labels, degraded copies; keep large files out of version control), `detector/`, `bench/`
