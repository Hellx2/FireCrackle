package org.firecracking.firelabel.core;

/** Cheap early rejection of world-space boxes that cannot be on screen. Conservative: never skips a visible box. */
public final class BoxCulling {
    private BoxCulling() {}

    /**
     * True only if the whole box is certainly not on screen: every corner is behind the camera, or every
     * corner is outside the same screen edge. Boxes crossing the camera plane are always kept.
     */
    public static boolean isOffScreen(Projector p, double[] cam, double[] min, double[] max) {
        if (min.length != 3 || max.length != 3 || min[0] > max[0] || min[1] > max[1] || min[2] > max[2]) {
            throw new IllegalArgumentException("invalid box");
        }
        int behind = 0;
        boolean allLeft = true;
        boolean allRight = true;
        boolean allAbove = true;
        boolean allBelow = true;
        for (int i = 0; i < 8; i++) {
            double[] pt = p.project(
                    ((i & 1) == 0 ? min[0] : max[0]) - cam[0],
                    ((i & 2) == 0 ? min[1] : max[1]) - cam[1],
                    ((i & 4) == 0 ? min[2] : max[2]) - cam[2]);
            if (pt == null) {
                behind++;
                continue;
            }
            allLeft &= pt[0] < 0;
            allRight &= pt[0] > p.width();
            allAbove &= pt[1] < 0;
            allBelow &= pt[1] > p.height();
        }
        if (behind == 8) {
            return true;
        }
        if (behind > 0) {
            return false; // crosses the camera plane: cannot tell cheaply, keep it
        }
        return allLeft || allRight || allAbove || allBelow;
    }
}
