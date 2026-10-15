package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class ProjectorTest {

    /** OpenGL-style perspective matrix, column-major: element (row r, col c) at index c * 4 + r. */
    static double[] perspective(double fovYDegrees, double aspect, double near, double far) {
        double f = 1.0 / Math.tan(Math.toRadians(fovYDegrees) / 2.0);
        double[] m = new double[16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) / (near - far);
        m[11] = -1.0;
        m[14] = 2.0 * far * near / (near - far);
        return m;
    }

    private static Projector standard() {
        return new Projector(perspective(90, 16.0 / 9.0, 0.05, 512), 1600, 900);
    }

    // ---------- basics ----------

    @Test
    void pointStraightAheadLandsInTheScreenCenter() {
        double[] p = standard().project(0, 0, -4);
        assertNotNull(p);
        assertEquals(800.0, p[0], 1e-6);
        assertEquals(450.0, p[1], 1e-6);
    }

    @Test
    void knownOffsetsLandOnKnownPixels() {
        // fovY 90: ndcY = y / 4, ndcX = x / (4 * 16/9)
        double[] p = standard().project(1, 1, -4);
        assertEquals(912.5, p[0], 1e-6);
        assertEquals(337.5, p[1], 1e-6); // +y is up, so it is ABOVE the center (smaller pixel y)
        double[] q = standard().project(-1, -1, -4);
        assertEquals(687.5, q[0], 1e-6);
        assertEquals(562.5, q[1], 1e-6);
    }

    @Test
    void screenOriginIsTopLeft() {
        // a point far up-left in view lands near pixel (0, 0), far down-right near (W, H)
        double[] tl = standard().project(-7.1111, 4, -4);
        assertEquals(0.0, tl[0], 0.5);
        assertEquals(0.0, tl[1], 1e-6);
        double[] br = standard().project(7.1111, -4, -4);
        assertEquals(1600.0, br[0], 0.5);
        assertEquals(900.0, br[1], 1e-6);
    }

    @Test
    void nearerPointsHaveSmallerDepth() {
        double near = standard().project(0, 0, -2)[2];
        double far = standard().project(0, 0, -20)[2];
        assertTrue(near < far);
        assertTrue(near > -1.0 && far < 1.0);
    }

    // ---------- failures ----------

    @Test
    void pointsBehindTheCameraAreRejected() {
        assertNull(standard().project(0, 0, 4));
        assertNull(standard().project(3, -2, 0.5));
    }

    @Test
    void pointOnTheCameraPlaneIsRejected() {
        assertNull(standard().project(1, 1, 0)); // w == 0
    }

    @Test
    void invalidMatrixOrSizeIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> new Projector(new double[15], 100, 100));
        assertThrows(IllegalArgumentException.class, () -> new Projector(new double[16], 0, 100));
        assertThrows(IllegalArgumentException.class, () -> new Projector(new double[16], 100, -1));
    }

    @Test
    void nanInputGivesNoPoint() {
        assertNull(standard().project(Double.NaN, 0, -4));
    }
}
