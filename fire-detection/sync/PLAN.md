# Sync code + OBS link: plan

## What it does

- **Sync strip (format v2).** The mod stamps every rendered frame with a small black/white barcode in the
  top-left corner that encodes **two** things: the game **tick** (ticks since launch, the number the label
  table and the jolt cracker use) and the mod's **render-frame id**. From the video alone you get the tick, so
  `phase = (tick - observed texture) % 32` and splice checks need no log. The frame id additionally keys the
  exact per-frame row in the mod's label log (anything that changes between rendered frames of one tick, such
  as fire screen positions). Works after re-encoding, dropped or duplicated frames, and rescaling.
  (v1 had only the frame id, 34 cells.)
- **OBS link.** A small controller that starts/stops OBS recording over obs-websocket v5 and reports
  the output file path, so one run = one video + one label log.

## Architecture (performance)

We process hours of video, so the hot path is Rust. Python is only used for the control plane.

- `core/` (Rust crate `fire_core`, no dependencies): strip encode/decode on raw 8-bit luma buffers,
  plus later the fire detector. A `decode` binary will read raw gray frames piped from `ffmpeg`
  and emit one frame id per video frame, with no per-frame allocation.
- `obs/` (Python, `obsws-python`): start/stop recording. Runs once per recording, so speed is irrelevant.
- Java mod: must produce the exact strip below.

## Streaming decoder: `fire-decode` (Rust binary in `core/`)

`fire-decode VIDEO [--labels labels.jsonl] [--out joined.jsonl]` turns a recording into
`video frame -> (tick, frame id) -> label row`, one JSONL line per decoded video frame:

`{"n":<video frame index>,"tick":T,"f":ID,"tick_ok":true,"label":<the label log row verbatim>}`

- Undecodable frame (no strip, too compressed): `"tick":null,"f":null,"tick_ok":null,"label":null`.
- Decoded but no such id in the label log: `"label":null`, `"tick_ok":null`.
- `tick_ok` = strip tick equals the label row's tick modulo 2^24. A false here means the video and the label log
  do not belong together (wrong log, splice, id wrap), the strongest integrity check the format gives us.
- A summary goes to stderr: frames, decoded, undecodable, matched, unmatched, tick mismatches, id repeats
  (duplicated frames) and id decreases (dropped or spliced footage).

Mechanics: `ffprobe` gives width/height; `ffmpeg` decodes with `-vsync 0` (no duplicated or dropped frames) and
crops to just the strip rows, `-pix_fmt gray`, raw frames on a pipe. The decoder reads exact-size frames into one
reusable buffer (no per-frame allocation, constant memory), so hours of video are fine. The label log is indexed by
byte offset (`id -> tick, offset`), not loaded into memory, and rows are fetched on demand.

Failure modes:
- ffmpeg/ffprobe missing or failing, unreadable video, width < 480 -> clear message and non-zero exit, never a panic.
- Truncated last frame or ffmpeg killed mid-run -> error that says how many frames were read, rows already
  written stay valid.
- ffmpeg frame-rate conversion duplicating/dropping frames would silently break the frame-to-row mapping -> `-vsync 0`.
- Pipe deadlock if ffmpeg's stderr fills while we only read stdout -> `-loglevel error -nostats`, stderr inherited.
- Label log with unparseable lines, duplicate ids (24-bit wrap) or blank lines -> counted and reported, first
  occurrence wins, never a crash; CRLF line endings handled.
- Broken output pipe (`| head`) -> exits quietly.
- Rescaled video: the strip is located from the video width alone (same rule as the encoder), so a 720p or 4K
  upload decodes the same way.

Tests (before the implementation): unit tests for ffprobe output parsing, exact-frame reading (clean EOF vs
truncation), label head parsing and the offset index, row formatting and the summary counters; an integration test that
builds a real x264 video (with unstamped frames, a duplicated frame, a missing label and a wrong-tick label),
runs the binary and checks every output row plus the summary; error-path tests (missing file).

## Strip format (the Java mod must match this exactly)

58 cells in one row, each a square of `c = 8 * (video_width / 1920)` px (float, NOT rounded; min 2.0),
anchored at (0, 0). Cell `i` covers x in `[i*c, (i+1)*c)`; the strip is `round(58*c)` x `round(c)` px.
Fractional size matters: at 1280 wide `c = 5.33`, and rounding to 5 would drift ~11 px by the last cell.
Encoders draw cell `i` over pixels `[round(i*c), round((i+1)*c))`.

| cells | meaning |
|---|---|
| 0 | calibration white (255) |
| 1 | calibration black (0) |
| 2..25 | 24-bit **tick** (game ticks since launch, wrapped mod 2^24), MSB first (1 = white, 0 = black) |
| 26..49 | 24-bit **frame id**, MSB first |
| 50..57 | CRC-8 (poly 0x07, init 0, no reflection) of the 6 bytes `tick(3, big-endian) ++ frame id(3, big-endian)`, MSB first |

Decode: luma only (chroma gets destroyed by 4:2:0), average the central half of each cell, threshold at
the midpoint of the two calibration cells. Reject if the white/black calibration gap is < 60, if any data
cell lands within `gap/8` of the threshold (ambiguous; keeps noise from passing the CRC by luck, which
alone is a 1-in-256 event per frame), or if the CRC fails.

## Label log (written by the mod, one JSON object per rendered frame, JSONL)

`{"f": <frame id>, "tick": <game tick>, "fires": [{"frame": 0-31, "sheet": 0|1, "mirrored": bool, "bbox": [x0,y0,x1,y1], "dist": float}]}`

## How it could break (failure modes)

- Compression blur bleeds neighbouring cells into each other -> sample cell centers only, use big cells.
- Limited-range video (16-235), brightness shifts, crushed blacks -> threshold from calibration cells,
  never a fixed 128.
- Rescaled video (720p / 1440p / 4K, odd widths) -> cell size derived from frame width.
- Chroma subsampling -> luma only.
- Garbage decoded from a frame with no strip (menu, wrong video, all black/white) -> calibration gap
  check + CRC must return `None`, never a wrong id. (CRC-8 still lets 1 in 256 random patterns through;
  the label join must also check ids are monotonic/in range.)
- Frame id range: 24 bits ~ 77 h at 60 fps. Out-of-range encode = error, not silent wrap.
- Tick range: 24 bits ~ 9.7 days at 20 TPS. The strip encoder rejects out-of-range ticks; the **mod** wraps the
  tick with `& 0xFFFFFF` before encoding (a decreasing tick in the strip is the wrap marker; the log keeps the full tick).
- Tick and frame id swapped by mistake (both fields are 24 bits, so a swap still passes the CRC if the encoder and
  decoder disagree) -> tests use distinct non-symmetric values and assert each field comes back in its own slot.
- Frame too small for the strip -> `None`.
- OBS not running / wrong port / wrong password -> clear error, not a stack trace.
- Start while already recording, stop while not recording -> explicit error, no double start.
- OBS may return no output path on stop -> surface it as an error.

## Tests (written before the implementation)

Rust (`core/tests/`): basics: round-trip for (tick, id) pairs incl. 0 and 0xFFFFFF in each field, many random
pairs; fields not swapped; known CRC-8 vector;
strip geometry; stamping leaves the rest of the frame untouched; stamping over a noisy background.
Failures: native widths (720p/1080p/1440p/4K), frames rescaled after stamping, blur, noise, limited range,
low contrast -> None, flipped bit -> None, blank/uniform/random frames -> None, out-of-range ids,
frames too small, wrong buffer length. Integration: stamp -> real ffmpeg x264 (CRF 28, and downscaled)
-> decode; skipped with a message if ffmpeg is missing.
Python (`obs/tests/`): start, stop, status via a fake client; path returned on stop; OBS-down, double start,
stop-when-idle, missing path errors.
