package org.firecracking.firelabel.core;

/**
 * The sync strip (format v2): 58 cells = 2 calibration + 24-bit game tick + 24-bit render-frame id +
 * 8-bit CRC, in the top-left of the frame. Second implementation of the format in core/src/sync.rs;
 * see sync/PLAN.md. Both are tested against core/testdata/strip_vectors.txt.
 */
public final class StripPattern {
    public static final int CELLS = 58;
    public static final int MAX_TICK = 0xFF_FFFF;
    public static final int MAX_ID = 0xFF_FFFF;
    private static final double MIN_CELL = 2.0;

    private StripPattern() {}

    /** Cell size in pixels for a video of this width (fractional, 8 px at 1920 wide). */
    public static double cellSize(int width) {
        return 8.0 * width / 1920.0;
    }

    /** {@code {width, height}} of the strip in pixels, or null if cells would be under 2 px. */
    public static int[] stripSize(int width) {
        if (width <= 0) {
            return null;
        }
        double c = cellSize(width);
        if (c < MIN_CELL) {
            return null;
        }
        return new int[] {(int) Math.round(CELLS * c), (int) Math.round(c)};
    }

    /** The tick as stored in the strip: the low 24 bits (the full tick stays in the label log). */
    public static int wrapTick(long tick) {
        if (tick < 0) {
            throw new IllegalArgumentException("tick must not be negative: " + tick);
        }
        return (int) (tick & MAX_TICK);
    }

    /** White/black state of each of the 58 cells (true = white). */
    public static boolean[] cells(int tick, int frameId) {
        if (tick < 0 || tick > MAX_TICK) {
            throw new IllegalArgumentException("tick out of range: " + tick);
        }
        if (frameId < 0 || frameId > MAX_ID) {
            throw new IllegalArgumentException("frame id out of range: " + frameId);
        }
        int crc = Crc8.crc8(new byte[] {
            (byte) (tick >> 16), (byte) (tick >> 8), (byte) tick,
            (byte) (frameId >> 16), (byte) (frameId >> 8), (byte) frameId
        });
        // 56 bits, MSB first: tick (24), frame id (24), crc (8)
        long word = ((long) tick << 32) | ((long) frameId << 8) | crc;
        boolean[] cells = new boolean[CELLS];
        cells[0] = true;
        cells[1] = false;
        for (int i = 2; i < CELLS; i++) {
            cells[i] = ((word >> (55 - (i - 2))) & 1) == 1;
        }
        return cells;
    }

    /** Pixel range {@code {x0, x1}} (x1 exclusive) that cell {@code i} covers. */
    public static int[] cellRange(int width, int i) {
        if (stripSize(width) == null) {
            throw new IllegalArgumentException("frame too narrow for the strip: " + width);
        }
        if (i < 0 || i >= CELLS) {
            throw new IllegalArgumentException("cell index out of range: " + i);
        }
        double c = cellSize(width);
        return new int[] {(int) Math.round(i * c), (int) Math.round((i + 1) * c)};
    }

    /** First pixel row of the strip (true = white), one entry per pixel. */
    public static boolean[] row(int width, int tick, int frameId) {
        boolean[] cells = cells(tick, frameId);
        int[] size = stripSize(width);
        if (size == null) {
            throw new IllegalArgumentException("frame too narrow for the strip: " + width);
        }
        boolean[] row = new boolean[size[0]];
        for (int i = 0; i < CELLS; i++) {
            int[] r = cellRange(width, i);
            for (int x = r[0]; x < r[1]; x++) {
                row[x] = cells[i];
            }
        }
        return row;
    }
}
