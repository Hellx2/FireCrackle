package org.firecracking.firelabel.core;

/**
 * Matrix helpers. Convention notes from the real game (1.16.5): {@code Matrix4f.writeRowFirst} writes the OpenGL
 * COLUMN-major order (element at row r, column c at index c * 4 + r), despite its name. {@link Projector} also
 * takes column-major, so a buffer from the game can go straight into it.
 */
public final class Matrices {
    private Matrices() {}

    public static double[] transpose(double[] m) {
        if (m == null || m.length != 16) {
            throw new IllegalArgumentException("matrix must have 16 elements");
        }
        double[] out = new double[16];
        for (int r = 0; r < 4; r++) {
            for (int c = 0; c < 4; c++) {
                out[c * 4 + r] = m[r * 4 + c];
            }
        }
        return out;
    }

    /** The game's matrix buffer (column-major) as a row-major array (element at row r, column c at r * 4 + c). */
    public static double[] rowMajorFromGameBuffer(float[] columnMajor) {
        if (columnMajor == null || columnMajor.length != 16) {
            throw new IllegalArgumentException("matrix must have 16 elements");
        }
        double[] d = new double[16];
        for (int i = 0; i < 16; i++) {
            d[i] = columnMajor[i];
        }
        return transpose(d);
    }
}
