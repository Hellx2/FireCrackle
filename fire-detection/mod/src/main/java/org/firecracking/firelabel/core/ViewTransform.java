package org.firecracking.firelabel.core;

/**
 * Converts points from the game's view space back to world space, so geometry captured in view space (entity
 * fire) can still be ray-cast against the world. The game computes {@code view = R * (world - camera) + t}
 * where R is a rotation and t the small view-bobbing offset.
 */
public final class ViewTransform {
    private final double[][] r = new double[3][3];
    private final double[] t = new double[3];
    private final double[] cam;

    /** @param viewRowMajor the 4x4 view matrix, row-major (translation in elements 3, 7, 11) */
    public ViewTransform(double[] viewRowMajor, double[] cameraWorld) {
        if (viewRowMajor == null || viewRowMajor.length != 16) {
            throw new IllegalArgumentException("view matrix must have 16 elements");
        }
        if (cameraWorld == null || cameraWorld.length != 3) {
            throw new IllegalArgumentException("camera position must be {x, y, z}");
        }
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                r[i][j] = viewRowMajor[i * 4 + j];
            }
            t[i] = viewRowMajor[i * 4 + 3];
        }
        this.cam = cameraWorld.clone();
    }

    /** world = camera + R^T * (viewPoint - t), valid because R is a rotation. */
    public double[] toWorld(double[] viewPoint) {
        if (viewPoint == null || viewPoint.length != 3) {
            throw new IllegalArgumentException("point must be {x, y, z}");
        }
        double[] q = {viewPoint[0] - t[0], viewPoint[1] - t[1], viewPoint[2] - t[2]};
        double[] out = new double[3];
        for (int j = 0; j < 3; j++) {
            out[j] = cam[j] + r[0][j] * q[0] + r[1][j] * q[1] + r[2][j] * q[2];
        }
        return out;
    }
}
