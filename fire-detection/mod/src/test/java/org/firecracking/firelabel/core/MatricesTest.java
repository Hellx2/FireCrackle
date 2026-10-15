package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

/**
 * Regression for a bug found in the first in-game run: {@code Matrix4f.writeRowFirst} in 1.16.5 does NOT write
 * row-major data, it writes the OpenGL column-major order. The numbers below are the hand projection exactly as
 * the game printed them (2560x1369 window, fov 70): in a correct column-major perspective matrix the -1 that
 * makes w = -z sits at index 11 and the -0.1 depth term at index 14.
 */
class MatricesTest {

    /** As read from the game's buffer. */
    private static final float[] GAME_BUFFER = {
        0.7637f, 0, 0, 0,
        0, 1.4281f, 0, 0,
        0, 0, -1.0001f, -1.0f,
        0, 0, -0.1f, 0
    };

    private static double[] asDoubles(float[] f) {
        double[] d = new double[f.length];
        for (int i = 0; i < f.length; i++) d[i] = f[i];
        return d;
    }

    // ---------- basics ----------

    @Test
    void transposeSwapsRowsAndColumns() {
        double[] m = new double[16];
        for (int i = 0; i < 16; i++) m[i] = i;
        double[] t = Matrices.transpose(m);
        for (int r = 0; r < 4; r++)
            for (int c = 0; c < 4; c++) assertEquals(m[r * 4 + c], t[c * 4 + r]);
    }

    @Test
    void transposeTwiceIsTheOriginalAndDoesNotModifyTheInput() {
        double[] m = new double[16];
        for (int i = 0; i < 16; i++) m[i] = i * 1.5 - 3;
        double[] copy = m.clone();
        assertArrayEquals(m, Matrices.transpose(Matrices.transpose(m)));
        assertArrayEquals(copy, m);
    }

    @Test
    void gameBufferIsColumnMajorSoItGoesStraightIntoTheProjector() {
        Projector p = new Projector(asDoubles(GAME_BUFFER), 2560, 1369);
        double[] centre = p.project(0, 0, -4);
        assertEquals(1280.0, centre[0], 1e-6);
        assertEquals(684.5, centre[1], 1e-6);
        // ndcX = 0.7637 * 1 / 4 = 0.190925 -> sx = (0.190925 * 0.5 + 0.5) * 2560
        double[] right = p.project(1, 0, -4);
        assertEquals((0.7637 / 4 * 0.5 + 0.5) * 2560, right[0], 0.01);
        // +y is up: above the centre means a smaller pixel row
        assertTrue(p.project(0, 1, -4)[1] < 684.5);
    }

    @Test
    void readingTheBufferAsRowMajorGivesDifferentWrongPixels() {
        // what the bug did: transposing the (already column-major) buffer
        Projector wrong = new Projector(Matrices.transpose(asDoubles(GAME_BUFFER)), 2560, 1369);
        double[] w = wrong.project(1, 0, -4);
        double correctX = (0.7637 / 4 * 0.5 + 0.5) * 2560;
        assertTrue(w == null || Math.abs(w[0] - correctX) > 100, "the transposed matrix must not look right");
    }

    @Test
    void rowMajorFromGameBufferHasTheProjectionTermsAtTheRowMajorPlaces() {
        double[] rm = Matrices.rowMajorFromGameBuffer(GAME_BUFFER);
        assertEquals(-1.0, rm[14], 1e-6); // row 3, column 2
        assertEquals(-0.1, rm[11], 1e-6); // row 2, column 3
        assertEquals(0.7637, rm[0], 1e-6);
        assertEquals(1.4281, rm[5], 1e-6);
    }

    @Test
    void translationOfAViewMatrixEndsUpInTheRightColumnOfTheRowMajorForm() {
        // column-major buffer of a translation by (0.5, -0.25, 2): the offsets are the last four elements
        float[] buf = {1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0.5f, -0.25f, 2, 1};
        double[] rm = Matrices.rowMajorFromGameBuffer(buf);
        assertEquals(0.5, rm[3], 1e-9);
        assertEquals(-0.25, rm[7], 1e-9);
        assertEquals(2.0, rm[11], 1e-9);
    }

    // ---------- failures ----------

    @Test
    void wrongSizedInputIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> Matrices.transpose(new double[15]));
        assertThrows(IllegalArgumentException.class, () -> Matrices.transpose(null));
        assertThrows(IllegalArgumentException.class, () -> Matrices.rowMajorFromGameBuffer(new float[8]));
    }
}
