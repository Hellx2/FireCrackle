# Baseline detector (classical, oracle box)

Question answered: given the labelled on-screen box of one fire part, which of the 128 candidates
(sheet 0/1 x mirrored x row 0..31) is it? Localisation is NOT in scope yet (the box comes from the label).

## Method
- Templates: fire_0 / fire_1 (16x512 RGBA, binary alpha) -> 32 rows each -> flip for mirrored = 128 templates.
  Each template keeps its opaque bounding box (the label box hugs visible opaque texels, so a fully visible
  part's box ~ that bbox) and its luma on opaque pixels.
- For a candidate, resample the frame crop to the template's bbox size (area average of luma), then
  ZNCC over the template's opaque pixels. ZNCC ignores brightness gain/offset, so a dark veil should not matter much.
- Prediction = argmax score; ties -> lowest index (deterministic).
- Eval tool `fire-eval`: streams video as rgb24, reads the joined jsonl, scales label coords (label w x h) to the
  video size, classifies every part, prints accuracy by source / vis / veil / dist / box size.

## Failure modes (each has a test)
- degenerate box (empty, zero/negative size, NaN, fully off-frame) -> Err, never a guess
- box partly off-frame -> clamped, still works
- 1-2 px wide box -> works, no panic, no NaN
- constant (flat) crop -> zero variance: score is the worst value, not NaN; result still sorted
- frame buffer length != w*h*3 -> Err
- identical templates (rows that repeat) -> tie broken by lowest index, test excludes them from "must be unique"
- mirrored vs normal confusion on symmetric rows -> reported as such by eval, not hidden
- brightness: x0.5 darkening and additive noise keep the answer
- scale: 1x, 3x, 7x nearest-neighbour renders of a template are recognised
- block parts are unions of several quads (not one flat quad): known model mismatch, shows up in eval numbers
- JSON reader: strings with escapes, nested arrays, negative/float numbers, truncated input -> Err

## Result 1 (clip1, every 10th frame, 34.6k labelled parts, oracle boxes)
Exact 2.7%, top-3 5.5%, sheet+mirror 24.8%, row 6.9%. Chance is 0.8% / 2.3% / 25% / 3.1%: AT CHANCE in every bucket
(size, distance, visibility, veil, source), so it is the model, not the conditions. Synthetic tests pass, so the
matcher itself works on a clean single quad. Why the real data breaks it:
- a block part box is the union of ~12 overlapping quads at different angles/scales (nether fire fields stack many),
  not one upright 16x16 quad;
- the overlay box is only the visible bottom strip of a screen-wide quad, not the opaque bbox of a template;
- partly visible parts have a box that is a sub-window of the texture.
Next: label per-QUAD screen corners (schema v4) and match with a homography warp; pool evidence per frame because
f0 and f1 are global (one fire_0 row and one fire_1 row per frame for all parts).

---
# Stage 1 (revised): is there fire in this frame? (whole frame, no localisation)

Ground truth: label `fire` plus visible fire area = sum over parts of box area x vis (video px^2), so "barely visible"
frames are not counted as hard misses. Report precision/recall by area bucket.

Method: the fire textures use only 481 distinct opaque colours (a smooth orange -> yellow -> white ramp). Build a colour
lookup of every RGB within distance D of that palette (YouTube shifts colours by a few levels). A frame's score = number of
pixels in the lookup. Decision = score >= threshold (swept in eval).

Failure modes (each has a test): wrong buffer length / zero size -> Err; flat frame filled with a palette colour -> every
pixel counts; neutral grey / blue / green noise -> ~0; darkening to 0.8 and +-8 noise keep palette pixels inside D;
a pixel just outside D does not count and just inside does (exact edge); other orange things (lava, glowstone,
pumpkins) WILL count: expected false positives, measured by eval not hidden; sync-strip rows are black/white, count 0.

---
# Stage 1, round 2: structure features + veil reversal

Per frame (rows below the sync strip), computed on the frame as is AND on a "GUI-veil-reversed" copy, dumped to CSV
(`fire-features`) next to the truth (fire, visible area, nearest distance, veil), analysed offline for which feature
separates "fire >= area X" from "no fire".

Features: palette count (mask), bright-palette count (luma >= 170), 4-connected blobs of the mask (count of blobs >= 20 px,
largest, share of mask pixels in blobs >= 50 px), pixel-art texture noise inside the mask (mean |dluma| of horizontally
adjacent mask pixels), global brightness (mean luma, share of dark pixels), so veils show up.

Veil reversal: the vanilla screen background is fillGradient 0xC0101010 -> 0xD0101010, i.e. colour (16,16,16) with alpha
0.753 at the top row to 0.816 at the bottom row. Forward: out = c*a + p*(1-a). Reverse: p = (out - c*a)/(1-a), clamped.
The detector cannot know a screen is open, so both versions are scored; the analysis decides if reversal helps.

Failure modes (tests): reversing a forward-veiled frame returns the original within rounding (the 8-bit round trip loses
information: error <= 1/(1-a) levels, asserted); alpha 0 is identity; alpha >= 1 or NaN -> Err; clamped to 0..255; row
gradient is applied per row; blobs: 4-connectivity (diagonal pixels are NOT joined), blobs touching the border, empty and full
masks, min-area filter exact edge, size mismatch -> Err, a 1920x1080 mask finishes fast (no recursion / stack overflow);
edge stats: no adjacent mask pairs -> 0 not NaN; mask/length mismatch -> Err.
Not in this round (noted, not forgotten): colour clustering to separate fire from lava, fog/distance colour shift model.

## Result 2 (clip1, every 10th frame; 1,212 no-fire frames vs 1,592 with visible fire area >= 1000 px^2)
AUC per feature (0.5 = useless): largest palette blob 0.72, palette count 0.69, share of mask in blobs>=50 0.66,
brightness 0.64, texture roughness 0.64 (0.22 = inverted on veiled frames). With fire area >= 30k px^2: blob 0.81, count 0.80.
Fire further than 30 blocks: 0.47-0.53 (nothing works). Best single-threshold balanced accuracy 0.72.
Veil reversal applied blindly to every frame HURTS (all u_ features below r_): it brightens the non-fire world as much as
the fire. It only helps if we know a screen is open; veiled frames are 171 of 1592 positives and their raw count AUC is
already 0.83, so veils are not the main problem here.
Conclusion: whole-frame statistics are swamped by orange Nether scenery. Next: LOCAL (patch-level) detection using the label
boxes as per-pixel ground truth (colour clusters + texture inside patches), then pool patches to a frame decision.

---
# Stage 1, round 3: patch-level detector

Frame (below the sync strip) is cut into 16x16 px patches (partial patches at the right/bottom edge are dropped).
Per patch 12 features from the pixels (+ neighbours for `ring`):
pal (loose palette share), strict (strict palette share), bright (palette and luma>=170), white (luma>=235), dark (luma<40),
lum, lumstd, grad (mean |dx|+|dy| luma), big (share of horizontal steps >30), sat, warm (mean (R-B)), ring (mean `pal` of the
existing 8 neighbour patches: fire contrasting its surroundings shows as pal >> ring).

Truth per patch from the label boxes (video px): POSITIVE if a part box with vis >= 0.3 covers >= 50% of the patch;
NEGATIVE if no box of any vis comes within 8 px of the patch; otherwise SKIPPED (ambiguous edge).
The box is the hull of visible texels, so inside it some pixels are background: truth is noisy on purpose, not hidden.

Training (python, scikit-learn decision tree) on the first 60% of the clip, evaluation on the last 40% (temporal split:
neighbouring frames are near-duplicates). Tree exported as JSON, evaluated in Rust (`PatchModel`), pooled to a frame decision
(number of positive patches, largest 4-connected cluster of positive patches). Lava/other orange: it appears as negative
patches in the data, so the tree learns what separates them; we check per-feature separation lava-like vs fire patches.

Failure modes (tests): features of flat / palette-filled / white / dark patches have the exact expected values; partial edge
patches dropped; ring uses only existing neighbours; truth edges (exactly 50% covered, exactly 8 px away, low-vis box blocks
negative but does not make positive); tree model: leaf/threshold edge `<=` goes left, malformed model -> Err, cycles/out-of-range
children -> Err; pooling: no positives -> 0; diagonal patches are not one cluster.

## Result 3 (patch level; train frames 0..19.5k, held-out 19.5k..32.6k, 16x16 patches, 987k labelled patches)
Held-out patch AUC of single features: ring 0.87 (context: fire sits next to fire, not "contrast"), warm 0.82, pal 0.81, sat 0.72,
lumstd 0.63. Learned trees: depth 8 -> 0.75, depth 5 -> 0.81, depth 3 -> 0.86 (only warm + sat matter). Same trees evaluated on
alternating blocks of the SAME clip: depth 5 -> 0.87, gradient boosting -> 0.94 (0.77 across the time split). So the learned
models memorise the scene: clip1 is one Nether area and the late part differs. Depth 3 tree on the held-out patches:
precision 0.93, recall 0.22.
Frame decision from the depth-3 tree (653 held-out frames; no-fire 254, fire>=1000 px^2 350): AUC 0.79 (positives count) /
0.78 (largest cluster); fire>=10k px^2: 0.86. Largest cluster >= 12 patches: recall 51%, false positives 6%. Fire further than
15 blocks: ~0% recall. Lava/other-orange is NOT separately demonstrated: clip1 has too little of it to train or measure on.
Limiting factor is data diversity, not features: needs overworld day/night, snow, caves, soul fire, and lava / glowstone /
pumpkin / torch / shroomlight scenes as deliberate hard negatives.

---
# Stage 1, round 4: pure colour filter + outlier removal (no learning)

Per pixel: is it within D (RGB distance) of the fire palette? Several D (12, 24, 40). Outlier removal: morphological OPENING
with a (2r+1)x(2r+1) square (erode then dilate) so isolated / thin specks vanish and only solid orange areas survive; r in 0,1,2.
Frame score = largest surviving 4-connected blob (px) and total surviving px. Evaluated for BIG and NEAR fire only
(visible area >= 10k px^2, nearest source <= 15 blocks) versus frames with no fire, plus the looser area >= 1000 set.
Lava ignored for now (user decision).

Failure modes (tests): r=0 identity; isolated pixel removed; 2x2 block removed at r=1 but 3x3 block kept exactly; r=2 needs 5x5;
blob touching the frame border is NOT eroded away by the border (out of frame counts as set for erosion, clear for dilation);
thin 1-px line removed; two blobs joined by a 1-px bridge are separated; empty/full masks; size mismatch -> Err; full-HD speed.

## Result 4 (pure colour filter + opening, 3263 frames at stride 10, no learning)
All no-fire frames: best AUC 0.78 (big+near fire: area >= 10k px^2 and <= 15 blocks). The worst "false positives" are LAVA
(frame 14070: a 181k px blob; 5 of 6 spot-checked high-blob runs were lava, the sixth a held torch/blaze rod). Ignoring lava as
agreed, by dropping those 13 frame runs from the negatives (1212 -> 1012; runs picked from big blobs in no-fire frames then
spot-checked, so slightly circular; list in the analysis snippet below):
  runs 2200-2240 5780-5910 6890-6910 7060-7100 14020-14760 14950-14970 15070-15380 17470-17980 19670-19690 20090-20480
       29160-29180 29330-29350 29800-29980
big+near fire, tolerance D24: AUC 0.887 (no cleaning) -> 0.893 (opening r=2). Best point: recall 0.91 at 23% false positives.
Tolerance 12 is too tight (0.76-0.79), 40 no better than 24. Outlier removal helps by only ~0.006 AUC: speckle is not the
problem, other large orange areas are. area >= 1000 px^2: AUC 0.83.
Also seen: the label `area` proxy (box x vis) is inflated for the on-fire overlay (frames 24540-24560: area 168k, nothing
orange visible), so some "positives" are weak by label noise, not by detector failure.

---
# Stage 1, round 5: blob SHAPE to reject lava (lava frames stay in the negatives)

Mask = palette D24, opened r=2, blobs >= 100 px. Per blob: area, bbox w/h, fill (area / bbox area), compactness
(perimeter / sqrt(area): ~4 for a solid square, higher for ragged/holey), brightness mean/std, share of near-white, saturation,
horizontal roughness inside the blob. Hypothesis (user): lava = massive, continuous, well-defined patches; fire = ragged, holey,
flame-shaped.
Blob truth from the labels: FIRE if >= 50% of its pixels are inside a part box with vis >= 0.3; NON-FIRE if none of its pixels
is inside any box; else skipped. So lava, glowstone, items, blazes are all just non-fire blobs.
Evaluation: blob-level AUC per feature; shallow tree on the first 60% of frames (temporal split), held-out frames judged by
"any blob classified fire" against frames with no fire (all of them, lava included) and big+near fire.

Failure modes (tests): perimeter of 1 px = 4, 2x2 = 8, 3x3 = 12, ring with a hole counts the hole's edge, a blob touching the
frame edge does not count out-of-frame sides; label map: 0 background, blobs numbered in scan order, 4-connectivity (diagonals
split), bbox exact, areas sum to the mask size; mismatched sizes -> Err; full-HD speed.

## Result 5 (blob shape vs lava; 30,874 blobs >= 100 px; train frames <= 60%, held-out the rest; lava frames NOT removed)
Blob-level held-out AUC (fire vs non-fire blob): lumstd 0.91, rough 0.86, sat 0.79, lum 0.30 (fire is DARKER on average),
bw 0.59, area 0.56, compact 0.53, fill 0.50, bh 0.51. The "massive, well-defined patch" hypothesis does NOT hold: size and shape
(area, fill, perimeter) carry almost no signal here; what separates fire from lava/other orange is brightness VARIATION inside
the blob (fire is high-contrast pixel art, lava is smooth) plus saturation.
Frame level, held-out frames (547 no fire incl. all lava vs 593 big+near fire): largest blob area alone AUC 0.86 (recall .97
at 36% false positives); depth-2 tree on blob features AUC 0.94 (recall .83, false-pos .05); depth-3 AUC 0.95 (recall .96,
false-pos .09); depth-4 0.96. Tree uses sat, lum, lumstd, white.
Caveats: one clip, one temporal split; the tree is tiny (4 features) but still fitted on this clip's lava and fire.

## Result 6 (small / far fire; stride 5, held-out frames after 60% of the clip; depth-3 tree on blob features, lava kept in negatives)
Settings compared (blob dump): C = opening r2, blobs >= 100 px (current); A = r1, >= 30 px; B = r0, >= 20 px. Finer settings do NOT help:
C false-pos 9% / A 8% / B 27% at the big+near-optimal threshold; AUC big+near 0.94 / 0.90 / 0.81.
Config C per class (AUC; recall at the one big+near threshold): big+near 0.94; 0.94 | small (1k-10k px^2) 0.69; 0.53 |
far (>15 blocks, >=1k px^2) 0.57; 0.34 | small+far 0.53; 0.27 | tiny (100-1k px^2) 0.46; 0.15.
Recall by visible area: 100-1k 15%, 1-2k 33%, 2-5k 44%, 5-10k 79%, 10-30k 84%, >=30k 96%.
Recall by nearest distance: 0-5 blocks 91%, 5-10 92%, 10-15 73%, 15-25 26%, 25-40 15%, 40+ 7%. False positives 9% throughout.
Conclusion: the colour+blob-texture detector is reliable to ~10-15 blocks and >= ~5000 px^2; below that the blob has too few
pixels for texture statistics and merges with orange scenery. Smaller/farther needs a different signal (e.g. temporal flicker:
fire's texture changes every game tick) or higher resolution, not finer colour settings.
Process note: bench/tests/test_blob_analysis.py::pos_sets tests were written in the same step as pos_sets and not run red first.

---
# Stage 1, round 6: temporal flicker (small / far fire)

The fire sprite advances one frame per game tick; the video runs about 3 frames per tick. Pair each sampled frame B (tick k) with the
latest earlier frame A of tick k-1 (needs tick_ok). Flicker mask = pixels that are palette-coloured in BOTH A and B and whose luma
changes by >= 60 (fire pixels jump between orange shades; static orange such as glowstone or shroomlight does not change; camera
motion mostly turns orange into non-orange, which "palette in both" excludes). Camera motion level = mean |dluma| over pixels that are
palette in neither frame (every 4th pixel), so flicker can be judged separately for a still and a moving camera.
Features per frame: bg_motion, flicker px (total), flicker px inside 4-connected blobs >= 6 px, largest flicker blob.

Failure modes (tests): identical frames -> empty mask; a palette pixel changing luma by exactly the threshold counts (>=), slightly
less does not; a pixel palette in only one frame does not count; non-palette pixels never count; rows above y0 are left out;
bg_motion excludes palette pixels, is 0 for identical frames and 0 (not NaN) when every pixel is palette; sampling step;
size mismatch / y0 past the end / NaN threshold -> Err; partner search: latest earlier entry of tick-1, none when the previous
tick is missing (a gap) or only the same tick is present, tick 0 has no predecessor (no underflow), several frames of one tick
pick the LATEST.

## Result 7 (temporal flicker, stride 5, 6281 paired frames, partner = latest frame of the previous tick, median gap 2 frames)
AUC vs no-fire frames, any flicker feature: big+near 0.82-0.85 (no better than the colour blob detector), small 0.57-0.66, far 0.48-0.51,
small+far 0.46-0.49, tiny 0.45-0.53. Restricting to the calmest quarter of frames (camera nearly still) does not rescue small/far
(0.47-0.51) and lowers big+near to 0.74. NEGATIVE RESULT: with these features flicker adds nothing for small/far fire. Not tested: motion
compensation, per-region flicker (instead of whole-frame totals), flicker restricted to blobs the colour detector already found.

# Stage 1, round 7: first-person overlay sprite matching (preliminary)

The overlay has known screen geometry (two fire_1 quads, view = identity, perspective with the user's FOV). core/src/overlay.rs projects
both quads, samples the frame where each opaque texel of a candidate row lands (quad 2 drawn over quad 1), drops samples where the
template is orange but the frame is not (hotbar / hearts / menus), and scores luma ZNCC plus a Fisher z that is comparable across rows
(only 10-45 texels per row are on screen: the overlay is mostly below the frame).
Tests (12, written first, run red before the code): hand-computed corner coordinates, mirror symmetry of the quads, every row of a
synthetic render recognised, darkening, hotbar, noise, flat frame, wrong FOV, input errors. Tuning of two test thresholds after the first
run (noise ceiling by z instead of raw r; hotbar size made realistic) is recorded in the commit history of this file's tests, not hidden.
## Result 8 (clip1, 194 overlay frames of 6526 sampled)
Row accuracy by guessed FOV: 60 -> 12%, 70 -> 35%, 80 -> 7%, 90 -> 6%, 100 -> 2%, 110 -> 8%. A sharp peak at 70 says the geometry is roughly right
(FOV 70 is the vanilla default; options.txt does not record it reliably). With overlay visibility >= 0.3: 61% (38 frames); < 0.3: 28%.
No systematic offset in the wrong answers (pred - truth mod 32 peaks at 0). As an "is the player on fire" detector it is NOT the clean
0-false-negative result expected: z AUC 0.86, at best recall 0.88 with 32% false positives.
Not yet done: compare the predicted visible hull of the quad with the label's overlay box (would show a geometry error directly);
the overlay is drawn with alpha 0.9, so 10% of the scene shows through (not modelled).

---
# Round 7b: how the first-person fire overlay really renders (read from the 1.16.5 Yarn sources) and a matcher that works

Source: net/minecraft/client/gui/hud/InGameOverlayRenderer.renderFireOverlay and GameRenderer.renderHand/getFov.
- Drawn in the hand pass AFTER `matrices.pop()`: view bobbing and hurt tilt are NOT applied, the matrix is identity. The quad geometry is fixed.
- Projection = getBasicProjectionMatrix(camera, tickDelta, changingFov=false): vertical FOV is HARDCODED 70 degrees (options.fov is not
  used; 60 when the camera is submerged in a fluid; shrinks while dead), aspect = framebuffer w / h. So the screen footprint depends only on
  the window aspect ratio. The earlier FOV sweep peaking at 70 was this.
- Two quads r = 0, 1 (s = 2r - 1): unit quad at z = -0.5, rotated s*10 degrees about Y, translated (-s*0.24, -0.3, 0). Quad 1 is drawn over quad 0.
- UVs: the sprite's four corners with a negligible inset (lerp by getAnimationFrameDelta = 4 / atlas size): c0 -> (maxU, maxV), c1 -> (minU, maxV),
  c2 -> (minU, minV), c3 -> (maxU, minV): the sprite appears horizontally mirrored.
- Colour (1, 1, 1, 0.9), default blend: 90% sprite over 10% scene. Depth func ALWAYS, no lighting. The sprite is the CURRENT animation frame (row f1).
- Afterwards the vignette darkens edges (the real frame is dimmer toward the bottom/sides: luma correlation suffers) and the GUI draws on top.
Luma ZNCC (round 7, row accuracy 35%) was wrong for this reason. The SILHOUETTE is robust: a pixel is orange where either quad's texel is opaque,
and scene-coloured elsewhere. Matcher: probe pixels on a grid, predict orange from each row's opacity (both quads), compare with the frame's
"warm" test (R >= 45, B*5 <= R, G*8 >= R), pick the row with the best agreement over probes whose prediction differs between rows.
Presence = Youden index of the best row (P(warm | predicted orange) - P(warm | predicted not orange)).
Python prototype on clip1 (194 overlay frames, 6332 sampled frames without): row accuracy 191/194 = 98.5% (vis >= 0.3: 100%, vis < 0.1: 100%);
presence at Youden >= 0.4: recall 99.5%, 0 false positives of 6332 (AUC 0.9997). The misses cluster around frames 26660-26890.

## Result 9 (Rust SilhouetteMatcher on clip1, stride 5; 194 overlay frames, 6332 without)
Row accuracy 191/194 = 98.5% (same as the Python prototype). Presence (Youden of the best row): AUC 0.9997; threshold 0.5 -> recall 98.5% (191/194),
0 false positives of 6332; threshold 0.4 -> recall 99.5%, 6 false positives; 0.3 -> 69 false positives. Tests: 11 in core/tests/silhouette.rs, written
first and run red (stubs) before the code. Two test-side bugs found and fixed on the way (not implementation tweaks): the synthetic renderer left unpainted
pixels inside texels (sub-sampling too sparse for 40 px texels), and "a perfect overlay has presence > 0.8" was wrong - white/yellow sprite texels are not
"warm", so a perfect render reaches ~0.5-0.9.
The 3 wrong frames (26660, 26795, 26805) sit in a stretch of lower agreement (0.78-0.88 vs 0.9+ elsewhere): each is predicted as row 5/6, and the
surrounding frames in the same stretch are right. Suspects (not yet tested): warm/lava scenery behind the transparent texels, or label/texture-upload timing at a tick boundary.
Remaining visibility limit: only the lower part of the sprite is on screen, rows differ in 6-30 texels there, so any occluder of a decisive texel can flip a row.

## Result 10 (chasing the 3 wrong frames: 26660, 26795, 26805)
Cause for 26795 / 26805: the player is standing inside a field of fire blocks, so the scene BEHIND the overlay is itself bright orange. Opaque and
transparent overlay texels look alike under the "warm" test (agreement falls to 0.78-0.88 in that stretch, and the wrong answers are rows 5/6). Rendering the
predicted overlay next to the real frame shows the overlay's silhouette is right; the real frame is simply orange everywhere.
Timing at tick boundaries is NOT the cause: neighbouring frames of the same stretch are right, and colour evidence recovers the labelled row in 26795 and 26805,
so the labels were correct there.
Fix: colour evidence. Where a row expects an opaque texel (quad 1 over quad 0), the frame should show that texel's colour up to a gain (vignette / veil); gain =
median of (sum observed / sum expected); a probe matches when no channel is more than 40 off; per-row score = matches - misses over probes that differ between rows
and that the row expects opaque. Combined ranking = z(silhouette agreement) + z(colour score) over the 32 rows (`ranked_colour`; `ranked` is the silhouette-only ranking
and presence is unchanged).
Clip1: silhouette only 191/194 (98.5%) -> combined 193/194 (99.5%); only 26660 remains (vis 0.18; it also has a held item over the lower part; cause not verified).
Python prototype sweep behind it (194 frames): colour alone 80% (tol 24) / 91% (32) / 95% (48); combined 99.5% at tol 40 weight 1, but 98.5% at tol 48 and 87% at
tol 64 with weight 2. CAVEAT: tolerance 40 and weight 1 were chosen on these same 194 frames and the gain over silhouette-only is 2 frames: encouraging, not proven.
Test note: `colour_evidence_beats_the_silhouette_over_an_orange_scene` first asserted col >= 28/32 (a guess made before any number). The first run gave 2/32 for the
silhouette and 19/32 for colour on that deliberately adversarial scene (uncorrelated random fire colours); the assertion was changed to "colour beats silhouette by >= 10 rows
and gets >= 16/32". That is a relaxation after seeing the result, kept visible here.

## Result 11 (clip2 = first genuinely unseen recording: 19,553 frames, 2560x1440 framebuffer, 60% fire frames, 932 sampled overlay frames)
All numbers with the detectors UNCHANGED from clip1 (blob tree trained on clip1 only, threshold fixed from clip1).
Blob detector, big+near fire: AUC 0.908, recall 81%, false positives 1.6% of 1558 no-fire frames (clip1 held-out: AUC 0.95, 96% / 9%). small 0.58, far 0.51,
small+far 0.51, tiny 0.59: still chance. It transfers for big, near fire; it is more conservative on the new clip.
Overlay matcher: row accuracy 75.5% (clip1: 99.5%), presence AUC 0.82 (clip1: 0.9997). Causes found, in order of size:
1. LAVA stretch (frames ~13400-15200, 271 overlay frames): 33% row accuracy. The camera is submerged in lava, scene = solid red fog, which looks "warm" and
   within colour tolerance of dark orange texels, so neither silhouette nor colour separates opaque from transparent texels.
2. GUI veil (pause menu / inventory) on 126 of 932 overlay frames: 36.5% -> 44.4% after detecting and reversing the veil (core/src/veil.rs, 11 tests written first;
   user-supplied formula: out = 16a + p(1-a), a = 192/255 at the top row to 208/255 at the bottom row). Frames outside the lava stretch: veiled 90.2%, not veiled 94.8%.
3. FOV: NOT the cause. From the game's own overlay box in the labels: clip1 overlay FOV = 70 in 196/196 frames; clip2 = 70 in 853 and 60 in 77 (lava). The user changed the
   options FOV repeatedly; it did not move the overlay (the hand/overlay projection ignores options.fov; GameRenderer.getFov(changingFov=false) = 70, x60/70 when submerged).
   Trying both 70 and 60 and choosing by evidence: 75.8% vs 76.6% always-70 vs 76.7% with the true FOV: no gain. Only 70 and 60 are tried (user: other versions later).
Outside the lava stretch (661 frames): 94.4%; the remaining errors are mostly off by 1-3 rows.
