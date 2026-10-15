package org.firecracking.firelabel.core;

/** Segment vs axis-aligned box (slab method), for letting entity hitboxes block the line of sight. */
public final class RayBox {
    private RayBox() {}

    /**
     * Fraction {@code t} in [0, 1] along {@code start -> end} where the segment first is inside the box
     * (0 if it starts inside), or -1 if the segment never touches the box. Non-finite input is a miss.
     */
    public static double entryFraction(double[] start, double[] end, double[] min, double[] max) {
        if (start == null || end == null || min == null || max == null
                || start.length != 3 || end.length != 3 || min.length != 3 || max.length != 3) {
            throw new IllegalArgumentException("all arguments must be {x, y, z}");
        }
        for (int i = 0; i < 3; i++) {
            if (min[i] > max[i]) {
                throw new IllegalArgumentException("min must not exceed max");
            }
            if (!Double.isFinite(start[i]) || !Double.isFinite(end[i])) {
                return -1;
            }
        }
        double near = 0;
        double far = 1;
        for (int i = 0; i < 3; i++) {
            double d = end[i] - start[i];
            if (d == 0) {
                if (start[i] < min[i] || start[i] > max[i]) {
                    return -1; // parallel to this slab and outside it
                }
            } else {
                double t1 = (min[i] - start[i]) / d;
                double t2 = (max[i] - start[i]) / d;
                if (t1 > t2) {
                    double tmp = t1;
                    t1 = t2;
                    t2 = tmp;
                }
                near = Math.max(near, t1);
                far = Math.min(far, t2);
                if (near > far) {
                    return -1;
                }
            }
        }
        return near;
    }
}
