package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

/** Segment vs axis-aligned box, used to let entity hitboxes block the line of sight to fire. */
class RayBoxTest {

    private static final double[] MIN = {-1, -1, -1};
    private static final double[] MAX = {1, 1, 1};

    private static double hit(double[] a, double[] b) {
        return RayBox.entryFraction(a, b, MIN, MAX);
    }

    // ---------- basics ----------

    @Test
    void segmentThroughTheMiddleEntersAtTheNearFace() {
        // from x = -5 to x = +5 along the x axis: enters at x = -1, which is 4 of 10 units along
        assertEquals(0.4, hit(new double[] {-5, 0, 0}, new double[] {5, 0, 0}), 1e-12);
    }

    @Test
    void worksFromTheOtherSideAndOnADiagonal() {
        assertEquals(0.4, hit(new double[] {5, 0, 0}, new double[] {-5, 0, 0}), 1e-12);
        // from (-3,-3,-3) to (3,3,3): enters at the corner (-1,-1,-1), a third of the way along
        assertEquals(1.0 / 3.0, hit(new double[] {-3, -3, -3}, new double[] {3, 3, 3}), 1e-12);
    }

    @Test
    void segmentEndingInsideTheBoxStillHitsIt() {
        // the target (e.g. a fire quad) lies inside the hitbox: the line still enters it first
        double t = hit(new double[] {-5, 0, 0}, new double[] {0, 0, 0});
        assertEquals(0.8, t, 1e-12);
    }

    @Test
    void segmentStartingInsideReportsZero() {
        assertEquals(0.0, hit(new double[] {0, 0, 0}, new double[] {5, 0, 0}), 1e-12);
    }

    // ---------- failures: must report no hit ----------

    @Test
    void missesAreMinusOne() {
        assertEquals(-1.0, hit(new double[] {-5, 3, 0}, new double[] {5, 3, 0}), 0.0); // passes above
        assertEquals(-1.0, hit(new double[] {-5, 0, 0}, new double[] {-3, 0, 0}), 0.0); // stops short
        assertEquals(-1.0, hit(new double[] {3, 0, 0}, new double[] {5, 0, 0}), 0.0); // starts beyond
        assertEquals(-1.0, hit(new double[] {5, 0, 0}, new double[] {9, 0, 0}), 0.0); // moves away
    }

    @Test
    void segmentParallelToAFaceOutsideTheSlabIsAMiss() {
        assertEquals(-1.0, hit(new double[] {-5, 0, 2}, new double[] {5, 0, 2}), 0.0);
    }

    @Test
    void segmentParallelToAFaceInsideTheSlabStillHits() {
        assertEquals(0.4, hit(new double[] {-5, 0.5, 0.5}, new double[] {5, 0.5, 0.5}), 1e-12);
    }

    @Test
    void zeroLengthSegmentIsAHitOnlyInsideTheBox() {
        assertEquals(0.0, hit(new double[] {0, 0, 0}, new double[] {0, 0, 0}), 0.0);
        assertEquals(-1.0, hit(new double[] {4, 0, 0}, new double[] {4, 0, 0}), 0.0);
    }

    @Test
    void nonFiniteInputIsAMiss() {
        assertEquals(-1.0, hit(new double[] {Double.NaN, 0, 0}, new double[] {5, 0, 0}), 0.0);
        assertEquals(-1.0, hit(new double[] {-5, 0, 0}, new double[] {Double.POSITIVE_INFINITY, 0, 0}), 0.0);
    }

    @Test
    void badArraysAreRejected() {
        assertThrows(IllegalArgumentException.class,
                () -> RayBox.entryFraction(new double[2], new double[] {1, 1, 1}, MIN, MAX));
        assertThrows(IllegalArgumentException.class,
                () -> RayBox.entryFraction(new double[3], new double[3], new double[] {1, 0, 0}, new double[] {0, 1, 1}));
    }
}
