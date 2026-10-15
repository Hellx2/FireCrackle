package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class BoxCullingTest {

    private static final double[] ORIGIN = {0, 0, 0};

    private static Projector projector() {
        return new Projector(ProjectorTest.perspective(90, 16.0 / 9.0, 0.05, 512), 1600, 900);
    }

    private static boolean off(double[] cam, double[] min, double[] max) {
        return BoxCulling.isOffScreen(projector(), cam, min, max);
    }

    // ---------- basics ----------

    @Test
    void boxInViewIsKept() {
        assertFalse(off(ORIGIN, new double[] {-1, -1, -5}, new double[] {1, 1, -3}));
    }

    @Test
    void boxFarToTheRightIsSkipped() {
        assertTrue(off(ORIGIN, new double[] {20, -1, -5}, new double[] {22, 1, -3}));
    }

    @Test
    void boxFarToTheLeftAboveAndBelowAreSkipped() {
        assertTrue(off(ORIGIN, new double[] {-22, -1, -5}, new double[] {-20, 1, -3}));
        assertTrue(off(ORIGIN, new double[] {-1, 20, -5}, new double[] {1, 22, -3}));
        assertTrue(off(ORIGIN, new double[] {-1, -22, -5}, new double[] {1, -20, -3}));
    }

    @Test
    void boxEntirelyBehindTheCameraIsSkipped() {
        assertTrue(off(ORIGIN, new double[] {-1, -1, 3}, new double[] {1, 1, 5}));
    }

    @Test
    void cameraPositionIsSubtracted() {
        double[] cam = {10, 0, 0};
        assertFalse(off(cam, new double[] {9, -1, -5}, new double[] {11, 1, -3}));
        assertTrue(off(cam, new double[] {30, -1, -5}, new double[] {32, 1, -3}));
    }

    // ---------- must stay conservative: never skip something that could be visible ----------

    @Test
    void boxStraddlingTheCameraPlaneIsKept() {
        assertFalse(off(ORIGIN, new double[] {-1, -1, -2}, new double[] {1, 1, 2}));
    }

    @Test
    void boxBiggerThanTheScreenIsKept() {
        // corners fall outside on BOTH sides, so no single edge excludes the whole box
        assertFalse(off(ORIGIN, new double[] {-50, -50, -5}, new double[] {50, 50, -3}));
    }

    @Test
    void boxJustTouchingTheScreenEdgeIsKept() {
        // ndcX = x / (4 * 16/9) at z = -4; the right edge is x = 7.111
        assertFalse(off(ORIGIN, new double[] {7.0, -1, -4.0001}, new double[] {9, 1, -3.9999}));
    }

    @Test
    void invalidBoxIsRejected() {
        assertThrows(IllegalArgumentException.class,
                () -> off(ORIGIN, new double[] {1, 0, 0}, new double[] {0, 1, 1}));
        assertThrows(IllegalArgumentException.class,
                () -> off(ORIGIN, new double[] {0, 0}, new double[] {1, 1, 1}));
    }
}
