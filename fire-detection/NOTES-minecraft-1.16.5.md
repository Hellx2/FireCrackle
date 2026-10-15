# Minecraft 1.16.5 fire internals (Yarn `1.16.5+build.10`)

Original behavioral notes checked locally against Minecraft 1.16.5, its vanilla assets and a
recorded label log. Names use Yarn `1.16.5+build.10` for reference; no decompiled game source
or mapping files are included here. The mod resolves its development dependencies through Gradle.

## 1. What drives the fire animation

- Fire is an **animated sprite in the block atlas**: `ModelLoader.FIRE_0` / `ModelLoader.FIRE_1`
  (`SpriteIdentifier`, `.getSprite()` gives the live `Sprite`).
- Chain per client tick: `MinecraftClient.tick()` -> `TextureManager.tick()` -> every tick listener
  -> `SpriteAtlasTexture.tick()` -> `tickAnimatedSprites()` -> `Sprite.tickAnimation()` for each animated sprite.
- **Gate**: texture updates run when a world exists, including while paused. Consequences:
  - Fire keeps animating while the game is paused (confirmed in game).
  - Fire does not animate on the title screen / while loading (`world == null`); confirmed: `f0` sat at 0 until the world loaded.
  - Leaving and re-entering a world pauses the animation for the time spent outside, so the phase shifts
    across a world leave/join (a legitimate discontinuity, not a splice).
- `Sprite.tickAnimation()` advances its elapsed counter, moves to the next animation step when
  that step's duration expires, wraps at the end, resets the elapsed counter and uploads the image.
  Both fire sheets have default frame time 1, so **the animation step advances once per client tick**.
- `frameIndex` is the **animation step**, not the image row. The displayed image row is
  `metadata.getFrameIndex(frameIndex)` (see section 2).

## 2. IMPORTANT: fire_0 uses a shifted frame order

`assets/minecraft/textures/block/fire_0.png.mcmeta` lists `frames: [16..31, 0..15]`; `fire_1.png.mcmeta` is
`{"animation": {}}` (plain 0..31).

So with step `s` (the sprite's `frameIndex`):

| sheet | displayed image row |
|---|---|
| `fire_0.png` | `(s + 16) % 32` |
| `fire_1.png` | `s` |

Within one session the two sheets are on the same step but **16 rows apart**. In the community notation
(`{frame} {sheet} {mirrored}`), the frame is the displayed row, so it depends on the sheet.
The mod's log must therefore record the displayed row, not the raw step (fixed in the mod, see section 6).

## 3. Phase

With a tick counter that increments on every client tick since launch: `(tick - step) % 32` is constant
(measured: one single value over 3471 in-world frames, including a 10-tick lag catch-up). The sum
`(tick + step) % 32` is not constant (16 distinct values). So the working phase formula is

    phase = (tick count - observed texture) % 32      (per sheet; fire_0 and fire_1 phases differ by 16)

The example table in the original write-up also only fits the subtraction form.
The constant value itself depends on when the world was joined (see section 1), it is not a fixed number.

## 4. Ticks, lag and frame timing

- Game logic runs at 20 ticks/s. `MinecraftClient.render` asks the `RenderTickCounter` for the number of ticks
  due this frame and runs `tick()` up to **min(10, due)** times before drawing. A lag spike therefore appears
  as the animation step jumping by up to 10 between two rendered frames (seen in the log: 1 -> 11).
- Rendering is not tied to ticks: roughly 3-5 rendered frames per tick at 60+ fps, so a texture stays on screen
  for several video frames. Two consecutive video frames can show the same image.
- The sprite is uploaded during the tick, so the texture on screen for a rendered frame is the one from the
  most recent tick. Reading the step at the end of the render (what the mod does) matches what was drawn.

## 5. Which sheet and which mirror is drawn (per fire block)

Vanilla `blockstates/fire.json` is a `multipart`. Each part has an `apply` **array** of models, which means
the game picks **one at random per block position** (weighted random, equal weights, seeded by position).

- Floor fire (no neighbours): `fire_floor0` (sheet 0) or `fire_floor1` (sheet 1). **No mirrored variant.**
- Side fire (rotated by `y` for each direction): `fire_side0`, `fire_side1`, `fire_side_alt0`, `fire_side_alt1`.
- Up fire: `fire_up0`, `fire_up1`, `fire_up_alt0`, `fire_up_alt1`.
- The `*0` models use `block/fire_0`, the `*1` models use `block/fire_1`.
- `*_alt*` models are the **mirrored** ones: their UVs are `[16,0,0,16]` instead of `[0,0,16,16]`
  (horizontal flip), nothing else differs.

So `{sheet, mirrored}` is not a runtime flag: it is **which baked model variant the position's seed selected**.
Per block that can be read from the chosen model, or reproduced by replaying the same seeded selection.
That is the planned next mod step (per-block sheet and mirror).

Note for the labels: the animation row shown is the same for every fire block on the same sheet at a given
tick; only sheet and mirror differ between blocks.

## 6. What the mod records now

`{"f":<frame id>,"tick":<client ticks since launch>,"f0":<displayed row of fire_0>,"f1":<displayed row of fire_1>}`
`-1` means the sprite could not be read. `tick` counts every client tick since launch (including title
screen and pause), so it matches the "ticks the game has been open" notion, and the phase is only comparable
within one stretch spent in a world.

The on-screen sync strip (format v2, see `sync/PLAN.md`) carries the same `tick` (low 24 bits) plus the frame
id `f`, so a video can be checked without the log: read the tick from the strip, the fire row from the
picture, and compute the phase. The frame id still keys the exact per-frame log row.

## 7a. Rendering facts needed for per-frame labels (verified in the real game)

- **`Matrix4f.writeRowFirst(FloatBuffer)` writes COLUMN-major** (OpenGL order), despite the name. In a perspective
  matrix the `-1` (w = -z) is at buffer index 11 and the depth term at 14. Pinned by `MatricesTest`, which uses the
  hand projection exactly as the game printed it. Getting this wrong transposes every projection and gives huge,
  wrong boxes.
- Matrices to use: `WorldRenderer.render(MatrixStack, tickDelta, limitTime, outline, Camera, GameRenderer,
  Lightmap, Matrix4f projection)`: view = `matrices.peek().getModel()` (rotation + bobbing, camera-relative),
  projection = the last parameter. The hand/overlay pass uses a different projection, loaded with
  `GameRenderer.loadProjectionMatrix`; 1.16.5 has no projection getter, so that call is hooked.
- The first-person overlay is drawn with an identity hand matrix (renderHand resets it), fire_1 only.
- `RaycastContext` needs a non-null entity (ShapeContext.of(entity) throws on null).
- Fire blocks are rendered in the cutout layers with back-face culling; the framebuffer is the window size
  (e.g. 2560x1369), not a nominal 1080p.

## 7. Environment gotchas (for building)

- Loom 1.14.10 needs Gradle 9.2+; the project uses the 9.2.1 wrapper.
- Gradle (any Java NIO `Selector`) fails inside Claude's own shell on this machine; run it from your own terminal.
- `genSources` fails while the dev client is running (jar locked); close the game first, or use `javap -c`.
