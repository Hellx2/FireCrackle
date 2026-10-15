package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

/**
 * The first-person fire overlay (InGameOverlayRenderer.renderFireOverlay): for r in {0, 1} a unit quad at
 * z = -0.5, translated by (-(2r-1) * 0.24, -0.3, 0) and rotated (2r-1) * 10 degrees about Y, in the hand's view space.
 * UV: (-.5,-.5) -> (max U, max V), (.5,-.5) -> (min U, max V), (.5,.5) -> (min U, min V), (-.5,.5) -> (max U, min V).
 */
class OverlayQuadsTest {

    private static final double[] IDENTITY = {
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1
    };
    private static final double[] ORIGIN = {0, 0, 0};

    // ---------- basics ----------

    @Test
    void twoQuads() {
        assertEquals(2, OverlayQuads.build(IDENTITY).length);
    }

    @Test
    void firstQuadCornerIsAtTheHandComputedPosition() {
        // r = 0: translate (+0.24, -0.3, 0), rotate -10 deg about Y. Corner (-0.5, -0.5, -0.5):
        //   x' = x cos(-10) + z sin(-10) = -0.40558, z' = -x sin(-10) + z cos(-10) = -0.57923, y' = -0.5
        double[] v0 = OverlayQuads.build(IDENTITY)[0].vertex(0);
        assertEquals(-0.40558 + 0.24, v0[0], 1e-4);
        assertEquals(-0.5 - 0.3, v0[1], 1e-4);
        assertEquals(-0.57923, v0[2], 1e-4);
    }

    @Test
    void secondQuadIsTheMirrorImageOfTheFirst() {
        // mirroring in x swaps left and right corners: vertex 0 <-> 1 and 2 <-> 3
        int[] mirror = {1, 0, 3, 2};
        Quad a = OverlayQuads.build(IDENTITY)[0];
        Quad b = OverlayQuads.build(IDENTITY)[1];
        for (int i = 0; i < 4; i++) {
            double[] pa = a.vertex(mirror[i]);
            double[] pb = b.vertex(i);
            assertEquals(-pa[0], pb[0], 1e-9, "x of vertex " + i);
            assertEquals(pa[1], pb[1], 1e-9, "y of vertex " + i);
            assertEquals(pa[2], pb[2], 1e-9, "z of vertex " + i);
        }
    }

    @Test
    void quadsSitInFrontOfTheCamera() {
        for (Quad q : OverlayQuads.build(IDENTITY)) {
            for (int i = 0; i < 4; i++) {
                assertTrue(q.vertex(i)[2] < -0.3 && q.vertex(i)[2] > -0.8, "z " + q.vertex(i)[2]);
            }
        }
    }

    @Test
    void uvFollowsTheVanillaVertexOrder() {
        Quad q = OverlayQuads.build(IDENTITY)[0];
        assertArrayEquals(new double[] {1, 1}, q.uv(0));
        assertArrayEquals(new double[] {0, 1}, q.uv(1));
        assertArrayEquals(new double[] {0, 0}, q.uv(2));
        assertArrayEquals(new double[] {1, 0}, q.uv(3));
    }

    @Test
    void theTextureAppearsFlippedOnScreen() {
        // as drawn by the game, seen from the camera at the view-space origin
        for (Quad q : OverlayQuads.build(IDENTITY)) {
            assertTrue(q.isMirroredFrom(ORIGIN));
        }
    }

    @Test
    void handMatrixIsAppliedAfterTheQuadTransform() {
        double[] shifted = {
            1, 0, 0, 0.5,
            0, 1, 0, 0,
            0, 0, 1, 0,
            0, 0, 0, 1
        };
        double[] plain = OverlayQuads.build(IDENTITY)[0].vertex(2);
        double[] moved = OverlayQuads.build(shifted)[0].vertex(2);
        assertEquals(plain[0] + 0.5, moved[0], 1e-9);
        assertEquals(plain[1], moved[1], 1e-9);
        assertEquals(plain[2], moved[2], 1e-9);
    }

    // ---------- failures ----------

    @Test
    void badMatrixIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> OverlayQuads.build(new double[15]));
        assertThrows(IllegalArgumentException.class, () -> OverlayQuads.build(null));
    }
}
