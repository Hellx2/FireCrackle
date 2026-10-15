# Capture plan: ~15 minutes of footage for the classical-algorithm baseline

## Is 15 minutes fair?

Yes, if it is 15 minutes **of fire**, planned for variety. Frame count is not the limit: 15 min at 60 fps is 54,000
frames, and every texture row repeats every 32 ticks (1.6 s), so a minute of fire already shows all 32 rows. Real
dev sessions logged 252,300 frames (70 min) but only 2% had fire (about 80 s), from 7 fire blocks and 2 mobs, nothing
beyond 20 blocks, and 17/32 overlay rows. What matters is the mix: distance, size, angle, lighting, sources, occlusion,
and things that look like fire but are not. Consecutive frames are near-duplicates, so the effective sample size is
much smaller than the frame count. Target: **at least 60% of frames with fire, 10-15% hard negatives.**

15 minutes is enough to build and *tune* a classical algorithm and to get a first honest accuracy number. It is not
enough to train a neural network (later: more footage, other worlds, resource packs).

## Settings that make the footage usable

Game (recording-safe client): `cd mod` then `.\gradlew.bat runClient -PfirelabelDebug=false`. The startup log must say
**"Recording-safe mode"**. The default `runClient` draws coloured debug boxes into the frame: never record that.

OBS:
- Source: **Game Capture** ("capture specific window"), or Window Capture with **Client Area** ticked. The strip must be
  the top-left pixels of the image: no title bar, no cropping, no filters, no overlays, no scaling in the scene.
- Canvas = output = the game's window size (no rescaling). Any size works (each label row stores the game's
  framebuffer size), but keep the window the **same size and un-minimised** for the whole recording.
- 60 fps. Encoder x264, rate control CRF 16-18 (or NVENC with CQP 16-18), High profile, Rec.709. Record to **MKV**
  (survives a crash), remux to MP4 afterwards if you like. Audio off. High quality on purpose: we degrade it ourselves
  with the calibrated YouTube-like chains, so the original must be clean.
- Hybrid-GPU laptops: OBS and the game must use the same GPU, or Game Capture shows black.
- Start recording **after** you join the world, stop **before** you leave. One game session = one label log
  (`mod/run/firelabel/labels-*.jsonl`); several OBS recordings in the same session all join to it by frame id.
- Name each file by scenario, e.g. `A-day-blocks.mkv`.

## Scenarios (about 15 minutes, fire on screen almost all the time)

| | Scenario | Min | Get |
|---|---|---|---|
| A | Fire blocks, daylight, open ground | 3 | ground/wall/ceiling fire (all variants), circle at 2-30 blocks, then fire lines out to 60 blocks |
| B | Dark: cave / night | 1.5 | same, low light, torches next to fire |
| C | Nether | 2.5 | `/fill` netherrack fire field with lava glow; 20 s of **soul fire** (a different texture: a hard negative) |
| D | Burning mobs | 2.5 | `/summon ... {Fire:32767}` cows, zombies, pigs, skeletons, spiders, a few at once; circle them; third person (F5); burning items |
| E | On-fire overlay | 1 | stand in fire with `/effect give @s fire_resistance 999`; look around, F5 and back |
| F | Occlusion | 1.5 | fire behind walls, glass, leaves; a mob in front of a fire; peeking round corners |
| G | Hard negatives (no fire) | 1.5 | lava, magma, torches, glowstone, redstone, campfire, fire-charge particles, sunsets, nether portal, red wool |
| H | Motion | 1 | fast turns, sprinting, rain, flying fast past fire |

Keep the HUD on for most of it (real cracking footage has it); do one short stretch with F1 (HUD hidden).

## Split without leaking

Adjacent frames are near-identical, so never split frames at random. Split **by scenario and time**: for each scenario,
the first ~65% of its ticks is **tuning**, the last ~35% is **held-out test** (the tick is on every frame, so this is a
filter on the joined file). Do not look at the held-out part while building the algorithm.

## After recording

```
core\target\release\fire-decode REC.mkv --labels mod\run\firelabel\labels-XXXX.jsonl --out data\REC.joined.jsonl
.venv\Scripts\python bench\coverage_report.py data\REC.joined.jsonl
```

The decoder summary must show nearly all frames decoded and `tick_mismatch=0`; the coverage report lists what is
still missing (distance, small fire, sources, rarely seen rows), so you know exactly what to re-record.
