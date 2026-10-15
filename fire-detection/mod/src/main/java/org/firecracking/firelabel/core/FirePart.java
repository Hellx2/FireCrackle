package org.firecracking.firelabel.core;

/**
 * One texture variant of a fire source: which sheet, whether it appears mirrored, which image row is
 * displayed, where it is on screen, and how much of it is visible.
 */
public final class FirePart {
    /** 0 = fire_0.png, 1 = fire_1.png. */
    public final int sheet;
    /** The texture appears flipped horizontally on screen. */
    public final boolean mirrored;
    /** Displayed image row (0..31) of that sheet. */
    public final int row;
    public final int visibleTexels;
    public final int opaqueTexels;
    /** Mean GUI alpha over the visible texels: 0 = nothing over it, 0.75 = a 75% dark veil (0..1). */
    public final double veil;

    private final BBox box;

    /** A part with no GUI over it. */
    public FirePart(int sheet, boolean mirrored, int row, BBox box, int visibleTexels, int opaqueTexels) {
        this(sheet, mirrored, row, box, visibleTexels, opaqueTexels, 0.0);
    }

    public FirePart(
            int sheet, boolean mirrored, int row, BBox box, int visibleTexels, int opaqueTexels, double veil) {
        if (Double.isNaN(veil) || veil < 0.0 || veil > 1.0) {
            throw new IllegalArgumentException("veil must be 0..1: " + veil);
        }
        if (sheet != 0 && sheet != 1) {
            throw new IllegalArgumentException("sheet must be 0 or 1: " + sheet);
        }
        if (row < 0 || row > 31) {
            throw new IllegalArgumentException("row must be 0..31: " + row);
        }
        if (box == null || box.isEmpty()) {
            throw new IllegalArgumentException("a part needs a non-empty box");
        }
        if (visibleTexels < 1) {
            throw new IllegalArgumentException("a part needs at least one visible texel: " + visibleTexels);
        }
        if (opaqueTexels < visibleTexels) {
            throw new IllegalArgumentException("visible texels exceed opaque texels: " + visibleTexels + " > " + opaqueTexels);
        }
        this.sheet = sheet;
        this.mirrored = mirrored;
        this.row = row;
        this.box = box.copy();
        this.visibleTexels = visibleTexels;
        this.opaqueTexels = opaqueTexels;
        this.veil = veil;
    }

    /** Visible opaque texels / opaque texels, 0..1. */
    public double vis() {
        return (double) visibleTexels / opaqueTexels;
    }

    /** A copy: callers cannot change the part's box. */
    public BBox box() {
        return box.copy();
    }
}
