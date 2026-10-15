# M4: timeline thumbnails (plan)

Goal: a picture strip along the timeline (like a video editor) so the user can see where scenes and fire are
without stepping. Decoding ten arbitrary frames costs seconds (each needs its own seek), so thumbnails come from
**one single ffmpeg pass over the keyframes only**, once per video, cached on disk.

## Design

- **Which frames**: keyframes only. `ffmpeg -skip_frame nokey -i VIDEO` decodes only the keyframes (a 1 h recording
  has a few thousand, decoding them is fast). `select=not(mod(n\,K))` thins them to at most
  `timeline.thumb_max_count`: `K = ceil(keyframes / max_count)`. One pass, stdout MJPEG, `showinfo` on stderr.
- **Matching is by timestamp, not by count**: every picture's time (`showinfo`, made absolute like the frame decoder
  does) is mapped to the nearest index frame and must be within `decode.align_tolerance` of that frame's interval.
  (The decoder's idea of a keyframe can differ from ffprobe's packet flag; a count check would reject good output.)
- **Cache** (`<video>.firecrackle/thumbs/`): `<frame>.jpg` per thumbnail plus `meta.json`
  `{fingerprint, width, quality, stride, frames}` written **last, atomically**. A store is valid only when the meta
  equals the expected key (same video fingerprint, width, quality, stride) and every listed file exists; otherwise
  it is stale and regenerated. A torn run (crash before `meta.json`) is therefore never half-trusted.
- **Background**: `Workspace::thumbs_start()` spawns one job thread (a second call while running does nothing);
  `thumbs_status()` = idle | running | ready | failed(message) | disabled, with a count. Dropping the workspace
  cancels the job (kills ffmpeg). A failure never blocks labelling.
- **Strip**: `thumbs::strip(frames, start, end, slots)` = for each slot across the visible range the nearest
  thumbnail frame. The UI draws each distinct thumbnail once per slot, so zooming in shows the nearest keyframe
  (coarse where keyframes are far apart: a stated limit, not a bug).
- **Settings** (all documented, ranges in `config.rs`): `timeline.thumb_width_px` (exists), `timeline.thumb_quality`
  (MJPEG q 2-31, default 5), `timeline.thumb_max_count` (default 600), `timeline.thumbs_enabled` (default on).
- Tauri: `thumbs_start`, `thumbs_status`, `thumb_strip`, `thumb` (raw bytes). UI: `lib/thumbs.ts` (slot geometry,
  pure), a thumbnail row in `Timeline.tsx`, polling status while running.

## Ways this can break (tests cover each)

- `stride`: 0 keyframes, fewer keyframes than the cap, exactly the cap, one over, max_count 1; selected frames
  always include the first keyframe, are strictly increasing, never exceed the cap.
- `assign`: times with a start offset, a time between frames, a time matching no frame (misaligned error naming it),
  two pictures mapping to one frame, empty list, non-increasing times.
- Store: missing file, corrupt/partial `meta.json`, other video (fingerprint), other width/quality/stride,
  frame not in the store, empty save, a failed write leaves the old store valid, atomic meta.
- `nearest`/`strip`: empty store, one thumbnail, ties, frames before the first / after the last thumbnail, a range
  of 0 frames, 0 slots, more slots than frames, `start >= end`.
- ffmpeg: missing tool, cancel before and during, a video with a single keyframe, a start offset (1.5 s),
  stride > 1, width bigger than the video (no upscale needed: `min(w, iw)`), odd sizes, paths with spaces.
- Workspace: persists across reopen (no regeneration), settings change makes it stale and regenerates, a failing
  source gives `failed` and a later start retries, two quick starts run once, drop while running does not hang,
  disabled does nothing, a source without thumbnail support gives `failed` not a panic, unknown frame is an error.
- UI (pure TS): slot geometry for any viewport/zoom/width, zero width, thumbnail wider than the view, frames
  outside the video, de-duplication of requested thumbnails.

## Tests first

Rust: `tests/thumbs.rs` (pure + real ffmpeg), `tests/thumbs_workspace.rs`, `tests/config.rs` (36 settings).
TS: `src/lib/thumbs.test.ts`. Red first, then implement; test files unchanged between red and green.
