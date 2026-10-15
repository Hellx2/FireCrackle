package org.firecracking.firelabel.core;

/**
 * One fire quad as it was drawn this frame (schema v4, see mod/PLAN.md): which sprite variant, the four corners projected to
 * framebuffer pixels (top-left origin, in the quad's own vertex order, or none when a corner is behind the camera), the
 * sprite-local uv of each corner, and how much of it is visible. Quads of a source are listed in draw order.
 */
public final class FireQuadLabel {
    /** 0 = fire_0.png, 1 = fire_1.png. */
    public final int sheet;
    /** The texture appears flipped horizontally on screen. */
    public final boolean mirrored;
    /** Displayed image row (0..31) of that sheet. */
    public final int row;
    public final int visibleTexels;
    public final int opaqueTexels;
    /** Mean GUI alpha over the visible texels (0..1). */
    public final double veil;

    private final double[][] corners; // null when any corner is behind the camera
    private final double[][] uv;

    public FireQuadLabel(
            int sheet, boolean mirrored, int row, double[][] corners, double[][] uv,
            int visibleTexels, int opaqueTexels, double veil) {
        if (sheet != 0 && sheet != 1) {
            throw new IllegalArgumentException("sheet must be 0 or 1: " + sheet);
        }
        if (row < 0 || row > 31) {
            throw new IllegalArgumentException("row must be 0..31: " + row);
        }
        if (visibleTexels < 1) {
            throw new IllegalArgumentException("a quad needs at least one visible texel: " + visibleTexels);
        }
        if (opaqueTexels < visibleTexels) {
            throw new IllegalArgumentException("visible texels exceed opaque texels: " + visibleTexels + " > " + opaqueTexels);
        }
        if (Double.isNaN(veil) || veil < 0.0 || veil > 1.0) {
            throw new IllegalArgumentException("veil must be 0..1: " + veil);
        }
        this.sheet = sheet;
        this.mirrored = mirrored;
        this.row = row;
        this.visibleTexels = visibleTexels;
        this.opaqueTexels = opaqueTexels;
        this.veil = veil;
        this.corners = corners == null ? null : copyPairs(corners, "corners");
        if (uv == null) {
            throw new IllegalArgumentException("uv is required");
        }
        this.uv = copyPairs(uv, "uv");
    }

    private static double[][] copyPairs(double[][] src, String what) {
        if (src.length != 4) {
            throw new IllegalArgumentException(what + " needs 4 points, got " + src.length);
        }
        double[][] out = new double[4][];
        for (int i = 0; i < 4; i++) {
            if (src[i] == null || src[i].length != 2) {
                throw new IllegalArgumentException(what + " point " + i + " must be {x, y}");
            }
            if (Double.isNaN(src[i][0]) || Double.isNaN(src[i][1]) || Double.isInfinite(src[i][0]) || Double.isInfinite(src[i][1])) {
                throw new IllegalArgumentException(what + " point " + i + " must be finite");
            }
            out[i] = src[i].clone();
        }
        return out;
    }

    /** Visible opaque texels / opaque texels, 0..1. */
    public double vis() {
        return (double) visibleTexels / opaqueTexels;
    }

    public boolean hasCorners() {
        return corners != null;
    }

    /** A copy of corner {@code i} as {@code {x, y}} pixels, or null if the quad has no on-screen corners. */
    public double[] corner(int i) {
        return corners == null ? null : corners[i].clone();
    }

    /** A copy of corner {@code i}'s sprite-local texture coordinate as {@code {u, v}}. */
    public double[] uv(int i) {
        return uv[i].clone();
    }
}
