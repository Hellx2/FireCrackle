package org.firecracking.firelabel.core;

/** Screen rectangle around world points: the region of the GUI mask worth reading back for one fire source. */
public final class ScreenBounds {
    private ScreenBounds() {}

    /**
     * Bounding box of the projected points, clamped to the screen. Empty if every point is behind the camera or
     * the box is entirely off screen. If only SOME points are behind the camera the projection cannot bound the
     * shape, so the whole screen is returned (conservative: it may only cause a bigger read, never a missed GUI).
     */
    public static BBox ofPoints(Projector p, double[] cam, double[][] points) {
        if (points == null) {
            throw new IllegalArgumentException("points are required");
        }
        BBox box = new BBox();
        int behind = 0;
        for (double[] pt : points) {
            if (pt == null || pt.length != 3) {
                throw new IllegalArgumentException("each point must be {x, y, z}");
            }
            double[] s = p.project(pt[0] - cam[0], pt[1] - cam[1], pt[2] - cam[2]);
            if (s == null) {
                behind++;
            } else {
                box.add(s[0], s[1]);
            }
        }
        if (points.length == 0 || behind == points.length) {
            return new BBox();
        }
        if (behind > 0) {
            BBox whole = new BBox();
            whole.add(0, 0);
            whole.add(p.width(), p.height());
            return whole;
        }
        return box.clamped(p.width(), p.height());
    }

    public static BBox ofBox(Projector p, double[] cam, double[] min, double[] max) {
        if (min == null || max == null || min.length != 3 || max.length != 3
                || min[0] > max[0] || min[1] > max[1] || min[2] > max[2]) {
            throw new IllegalArgumentException("invalid box");
        }
        double[][] corners = new double[8][];
        for (int i = 0; i < 8; i++) {
            corners[i] = new double[] {
                (i & 1) == 0 ? min[0] : max[0], (i & 2) == 0 ? min[1] : max[1], (i & 4) == 0 ? min[2] : max[2]
            };
        }
        return ofPoints(p, cam, corners);
    }
}
