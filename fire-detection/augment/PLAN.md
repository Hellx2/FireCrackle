# Augmentation tooling: plan

Facts and assumptions about YouTube's encoding are in `YOUTUBE_ENCODING.md`. This tool turns them into
reproducible, recorded degradations of our footage.

## Pieces (Python, control plane; the heavy lifting is ffmpeg)

- `profiles.py` (pure): the ladder, the bitrate ranges (ASSUMPTION, overridable by `profiles_calibrated.json`),
  `sample_chain(rng, source)` -> a `ChainSpec`, `stage_args(...)` -> ffmpeg argument lists, `manifest()`.
- `degrade.py`: `run_chain(input, out_dir, spec)` runs the stages with ffmpeg and writes the output plus a
  `*.manifest.json` (exact parameters, commands, ffmpeg version, seed). CLI: `python degrade.py IN OUT_DIR --seed N`.
- `calibrate.py`: `analyze(path)` -> codec, profile, level, resolution, fps, pixel format, color range, average
  bitrate, keyframe interval, B-frame ratio; `nearest_rung(stats)`. CLI prints a table for several files.
- `robustness.py`: stamps a synthetic video (Rust example `gen_stamped_video`), degrades it with many sampled chains,
  runs `fire-decode` and reports how often the sync strip still decodes, per codec and rung.

## Chain model

`source -> [recorder: x264 crf 16-23 | none] -> youtube-like encode at a ladder rung (h264 | vp9 | av1) -> output`

- Rungs: 2160, 1440, 1080, 720, 480, 360, 240, 144 (16:9 widths, even). Never above the source height; H.264 never
  above 1080 (YouTube's H.264 ladder stops there).
- Bitrate: sampled in the rung's range for the codec, multiplied by 1.5 for 48+ fps.
- Keyframe interval: 2 to 5 seconds (ASSUMPTION), B-frames 2, High profile, yuv420p, bt709 tags.
- Scaler: bicubic / lanczos / bilinear, chosen per chain.
- Same seed -> same chain, so a dataset can be regenerated exactly.

## How it could break (failure modes)

- Upscaling or odd dimensions make ffmpeg fail or produce junk -> ladder is capped at the source and widths are even.
- Frame count changes (dup/drop) would break the label join -> output must keep the input frame count; tested.
- A codec/encoder missing on another machine -> a clear error naming the encoder, not a Python traceback.
- Silent claims of precision -> every range is flagged ASSUMPTION in the manifest until calibrated.
- Non-determinism -> a single `random.Random(seed)`, no global state; tested.
- Bitrate ranges that exclude real YouTube values -> ranges are wide and contain the published anchor points; tested.
- Output too narrow for the strip (width < 480) -> manifest says `strip_decodable: false` instead of hiding it.
- Calibration file with bad or reversed ranges -> rejected on load with a message.

## Tests (before the implementation)

Pure: ladder/upscale rules, ranges, 60 fps multiplier, determinism, even dimensions and aspect, GOP range, argument
contents per codec, manifest content and JSON-ability, override loading and validation, invalid inputs.
Integration (needs ffmpeg): each codec chain on a small synthetic clip, checking codec, height, pixel format and
frame count with ffprobe; recorder stage; error paths. Calibrate: known GOP/B-frame settings are recovered.
