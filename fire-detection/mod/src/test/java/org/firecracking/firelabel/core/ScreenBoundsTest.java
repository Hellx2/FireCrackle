package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Arrays;
import org.junit.jupiter.api.Test;

/** Screen rectangle around a fire source: the region of the GUI mask that has to be read back. */
class ScreenBoundsTest {

    private static final double[] ORIGIN = {0, 0, 0};

    private static Projector projector() {
        return new Projector(ProjectorTest.perspective(90, 16.0 / 9.0, 0.05, 512), 1600, 900);
    }

    // ---------- basics ----------

    @Test
    void aBoxInViewGivesItsProjectedExtent() {
        // x, y in [-1, 1], z in [-5, -3]: the nearest face (z = -3) is the biggest on screen
        BBox b = ScreenBounds.ofBox(projector(), ORIGIN, new double[] {-1, -1, -5}, new double[] {1, 1, -3});
        assertEquals(650.0, b.x0(), 1e-6);
        assertEquals(300.0, b.y0(), 1e-6);
        assertEquals(950.0, b.x1(), 1e-6);
        assertEquals(600.0, b.y1(), 1e-6);
    }

    @Test
    void pointsGiveTheirBoundingBox() {
        BBox b = ScreenBounds.ofPoints(projector(), ORIGIN, new double[][] {{-1, -1, -4}, {1, 1, -4}});
        assertEquals(687.5, b.x0(), 1e-6);
        assertEquals(337.5, b.y0(), 1e-6);
        assertEquals(912.5, b.x1(), 1e-6);
        assertEquals(562.5, b.y1(), 1e-6);
    }

    @Test
    void theCameraPositionIsSubtracted() {
        BBox b = ScreenBounds.ofBox(projector(), new double[] {10, 0, 0}, new double[] {9, -1, -5}, new double[] {11, 1, -3});
        assertEquals(650.0, b.x0(), 1e-6);
        assertEquals(950.0, b.x1(), 1e-6);
    }

    @Test
    void theResultIsClampedToTheScreen() {
        // x in [6, 8.5] at z = -4 crosses the right edge (x = 7.111)
        BBox b = ScreenBounds.ofBox(projector(), ORIGIN, new double[] {6, -1, -4.0001}, new double[] {8.5, 1, -3.9999});
        assertEquals(1600.0, b.x1(), 1e-6);
        assertEquals(1475.0, b.x0(), 0.05); // the 0.0002 depth span moves the pixel by ~0.02
    }

    // ---------- conservative when it cannot tell ----------

    @Test
    void ifSomePointsAreBehindTheCameraTheWholeScreenIsUsed() {
        BBox b = ScreenBounds.ofBox(projector(), ORIGIN, new double[] {-1, -1, -2}, new double[] {1, 1, 2});
        assertEquals(0.0, b.x0());
        assertEquals(0.0, b.y0());
        assertEquals(1600.0, b.x1());
        assertEquals(900.0, b.y1());
    }

    @Test
    void entirelyBehindTheCameraOrOffScreenIsEmpty() {
        assertTrue(ScreenBounds.ofBox(projector(), ORIGIN, new double[] {-1, -1, 3}, new double[] {1, 1, 5}).isEmpty());
        assertTrue(ScreenBounds.ofBox(projector(), ORIGIN, new double[] {20, -1, -5}, new double[] {22, 1, -3}).isEmpty());
    }

    @Test
    void noPointsIsEmptyAndBadInputIsRejected() {
        assertTrue(ScreenBounds.ofPoints(projector(), ORIGIN, new double[0][]).isEmpty());
        assertThrows(IllegalArgumentException.class, () -> ScreenBounds.ofPoints(projector(), ORIGIN, null));
        assertThrows(IllegalArgumentException.class,
                () -> ScreenBounds.ofPoints(projector(), ORIGIN, new double[][] {{1, 2}}));
        assertThrows(IllegalArgumentException.class,
                () -> ScreenBounds.ofBox(projector(), ORIGIN, new double[] {1, 0, 0}, new double[] {0, 1, 1}));
    }
}
