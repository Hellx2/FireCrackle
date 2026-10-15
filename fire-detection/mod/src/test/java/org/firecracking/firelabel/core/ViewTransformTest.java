package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class ViewTransformTest {

    /** Row-major 4x4: rotation about Y then X, plus translation (the bobbing offset). */
    private static double[] view(double yawDeg, double pitchDeg, double tx, double ty, double tz) {
        double cy = Math.cos(Math.toRadians(yawDeg)), sy = Math.sin(Math.toRadians(yawDeg));
        double cx = Math.cos(Math.toRadians(pitchDeg)), sx = Math.sin(Math.toRadians(pitchDeg));
        double[][] ry = {{cy, 0, sy}, {0, 1, 0}, {-sy, 0, cy}};
        double[][] rx = {{1, 0, 0}, {0, cx, -sx}, {0, sx, cx}};
        double[][] r = new double[3][3];
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                for (int k = 0; k < 3; k++) r[i][j] += rx[i][k] * ry[k][j];
        return new double[] {
            r[0][0], r[0][1], r[0][2], tx,
            r[1][0], r[1][1], r[1][2], ty,
            r[2][0], r[2][1], r[2][2], tz,
            0, 0, 0, 1
        };
    }

    /** What the game does: view = R * (world - camera) + t. */
    private static double[] forward(double[] m, double[] cam, double[] w) {
        double x = w[0] - cam[0], y = w[1] - cam[1], z = w[2] - cam[2];
        return new double[] {
            m[0] * x + m[1] * y + m[2] * z + m[3],
            m[4] * x + m[5] * y + m[6] * z + m[7],
            m[8] * x + m[9] * y + m[10] * z + m[11]
        };
    }

    // ---------- basics ----------

    @Test
    void identityViewJustOffsetsByTheCamera() {
        double[] identity = view(0, 0, 0, 0, 0);
        double[] w = new ViewTransform(identity, new double[] {1, 2, 3}).toWorld(new double[] {0, 0, -5});
        assertArrayEquals(new double[] {1, 2, -2}, w, 1e-9);
    }

    @Test
    void quarterTurnAboutYIsUndone() {
        double[] m = view(90, 0, 0, 0, 0);
        double[] cam = {10, 0, 0};
        double[] w = {10, 0, -5};
        double[] v = forward(m, cam, w);
        assertEquals(-5, v[0], 1e-9); // sanity of the test's own forward model
        assertArrayEquals(w, new ViewTransform(m, cam).toWorld(v), 1e-9);
    }

    @Test
    void roundTripsRotationAndTranslation() {
        double[][] views = {view(35, -20, 0.01, -0.02, 0.005), view(-130, 60, 0, 0.1, 0), view(0, 89, 0.2, 0, -0.3)};
        double[] cam = {104.5, 63.25, -87.75};
        double[][] points = {{100, 64, -90}, {120, 70, -60}, {104.5, 63.25, -87.75}, {-5, 0, 3}};
        for (double[] m : views) {
            ViewTransform t = new ViewTransform(m, cam);
            for (double[] w : points) {
                assertArrayEquals(w, t.toWorld(forward(m, cam, w)), 1e-7);
            }
        }
    }

    @Test
    void doesNotModifyItsInput() {
        double[] m = view(10, 10, 0, 0, 0);
        double[] copy = m.clone();
        double[] p = {1, 2, 3};
        new ViewTransform(m, new double[] {0, 0, 0}).toWorld(p);
        assertArrayEquals(copy, m);
        assertArrayEquals(new double[] {1, 2, 3}, p);
    }

    // ---------- failures ----------

    @Test
    void badInputIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> new ViewTransform(new double[15], new double[3]));
        assertThrows(IllegalArgumentException.class, () -> new ViewTransform(new double[16], new double[2]));
        ViewTransform t = new ViewTransform(view(0, 0, 0, 0, 0), new double[3]);
        assertThrows(IllegalArgumentException.class, () -> t.toWorld(new double[2]));
    }
}
