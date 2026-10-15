package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class QuadTest {

    // Counter-clockwise from the front (the +z side): bottom-left, bottom-right, top-right, top-left.
    private static final double[][] FRONT = {{-1, -1, -4}, {1, -1, -4}, {1, 1, -4}, {-1, 1, -4}};
    private static final double[][] UV_NORMAL = {{0, 1}, {1, 1}, {1, 0}, {0, 0}};
    private static final double[][] UV_MIRRORED = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};

    private static double[][] shift(double[][] a, int by) {
        double[][] out = new double[4][];
        for (int i = 0; i < 4; i++) out[i] = a[(i + by) % 4];
        return out;
    }

    // ---------- basics ----------

    @Test
    void normalAndMirroredUvAreTold_Apart() {
        assertFalse(new Quad(FRONT, UV_NORMAL).isMirrored());
        assertTrue(new Quad(FRONT, UV_MIRRORED).isMirrored());
    }

    @Test
    void mirroredDoesNotDependOnWhichCornerTheQuadStartsAt() {
        for (int by = 0; by < 4; by++) {
            assertFalse(new Quad(shift(FRONT, by), shift(UV_NORMAL, by)).isMirrored(), "normal, start " + by);
            assertTrue(new Quad(shift(FRONT, by), shift(UV_MIRRORED, by)).isMirrored(), "mirrored, start " + by);
        }
    }

    @Test
    void quadFacingTheCameraIsFrontFacing() {
        assertTrue(new Quad(FRONT, UV_NORMAL).isFrontFacing(new double[] {0, 0, 0}));
        assertTrue(new Quad(FRONT, UV_NORMAL).isFrontFacing(new double[] {3, -2, 10}));
    }

    // ---------- failures ----------

    @Test
    void reversedWindingIsBackFacing() {
        double[][] reversed = {FRONT[1], FRONT[0], FRONT[3], FRONT[2]};
        assertFalse(new Quad(reversed, new double[][] {{1, 1}, {0, 1}, {0, 0}, {1, 0}}).isFrontFacing(new double[] {0, 0, 0}));
    }

    @Test
    void cameraBehindTheQuadPlaneSeesItsBack() {
        assertFalse(new Quad(FRONT, UV_NORMAL).isFrontFacing(new double[] {0, 0, -9}));
    }

    @Test
    void cameraExactlyOnThePlaneIsNotFrontFacing() {
        assertFalse(new Quad(FRONT, UV_NORMAL).isFrontFacing(new double[] {5, 5, -4}));
    }

    @Test
    void degenerateUvIsNotMirrored() {
        double[][] same = {{0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}};
        assertFalse(new Quad(FRONT, same).isMirrored());
    }

    // ---------- appearance as seen from the camera (entity/overlay quads are not wound like block quads) ----------

    private static final double[] CAM = {0, 0, 0};

    private static double[][] reversed(double[][] a) {
        return new double[][] {a[1], a[0], a[3], a[2]};
    }

    @Test
    void mirroredFromMatchesTheModelFlagForFrontFacingQuads() {
        assertFalse(new Quad(FRONT, UV_NORMAL).isMirroredFrom(CAM));
        assertTrue(new Quad(FRONT, UV_MIRRORED).isMirroredFrom(CAM));
    }

    @Test
    void clockwiseQuadWithTheSamePhysicalMappingIsNotMirroredEitherWay() {
        // same quad and same texture placement, vertices listed the other way round
        assertFalse(new Quad(reversed(FRONT), reversed(UV_NORMAL)).isMirroredFrom(CAM));
        assertTrue(new Quad(reversed(FRONT), reversed(UV_MIRRORED)).isMirroredFrom(CAM));
    }

    @Test
    void mirroredFromFlipsWhenTheCameraSeesTheBack() {
        // camera behind the quad plane sees the texture reversed
        double[] behind = {0, 0, -9};
        assertTrue(new Quad(FRONT, UV_NORMAL).isMirroredFrom(behind));
        assertFalse(new Quad(FRONT, UV_MIRRORED).isMirroredFrom(behind));
    }

    @Test
    void degenerateQuadsAreNeverMirroredFrom() {
        double[][] point = {{0, 0, -4}, {0, 0, -4}, {0, 0, -4}, {0, 0, -4}};
        assertFalse(new Quad(point, UV_MIRRORED).isMirroredFrom(CAM));
        double[][] same = {{0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}};
        assertFalse(new Quad(FRONT, same).isMirroredFrom(CAM));
    }

    @Test
    void zeroAreaQuadIsNotFrontFacing() {
        double[][] point = {{0, 0, -4}, {0, 0, -4}, {0, 0, -4}, {0, 0, -4}};
        assertFalse(new Quad(point, UV_NORMAL).isFrontFacing(new double[] {0, 0, 0}));
    }
}
