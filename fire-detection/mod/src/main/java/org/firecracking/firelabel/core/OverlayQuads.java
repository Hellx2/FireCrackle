package org.firecracking.firelabel.core;

/**
 * Geometry of the first-person on-fire overlay (vanilla {@code InGameOverlayRenderer.renderFireOverlay}), in the
 * hand's view space. For r in {0, 1}, with s = 2r - 1: a unit quad at z = -0.5 is rotated s * 10 degrees about Y,
 * translated by (-s * 0.24, -0.3, 0), then transformed by the hand view matrix. The fire_1 sprite is mapped with
 * (-.5,-.5) -> (max U, max V), (.5,-.5) -> (min U, max V), (.5,.5) -> (min U, min V), (-.5,.5) -> (max U, min V),
 * which in sprite-local coordinates is the uv list below.
 */
public final class OverlayQuads {
    private static final double[][] CORNERS = {{-0.5, -0.5, -0.5}, {0.5, -0.5, -0.5}, {0.5, 0.5, -0.5}, {-0.5, 0.5, -0.5}};
    private static final double[][] UV = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};

    private OverlayQuads() {}

    /** @param handViewRowMajor the 4x4 model matrix at the renderFireOverlay call, row-major */
    public static Quad[] build(double[] handViewRowMajor) {
        if (handViewRowMajor == null || handViewRowMajor.length != 16) {
            throw new IllegalArgumentException("matrix must have 16 elements");
        }
        Quad[] quads = new Quad[2];
        for (int r = 0; r < 2; r++) {
            double s = 2 * r - 1;
            double tx = -s * 0.24;
            double angle = Math.toRadians(s * 10.0);
            double cos = Math.cos(angle);
            double sin = Math.sin(angle);
            double[][] v = new double[4][];
            for (int i = 0; i < 4; i++) {
                double x = CORNERS[i][0];
                double y = CORNERS[i][1];
                double z = CORNERS[i][2];
                // rotate about +Y, then translate
                double rx = x * cos + z * sin + tx;
                double ry = y - 0.3;
                double rz = -x * sin + z * cos;
                v[i] = new double[] {
                    handViewRowMajor[0] * rx + handViewRowMajor[1] * ry + handViewRowMajor[2] * rz + handViewRowMajor[3],
                    handViewRowMajor[4] * rx + handViewRowMajor[5] * ry + handViewRowMajor[6] * rz + handViewRowMajor[7],
                    handViewRowMajor[8] * rx + handViewRowMajor[9] * ry + handViewRowMajor[10] * rz + handViewRowMajor[11]
                };
            }
            quads[r] = new Quad(v, UV);
        }
        return quads;
    }
}
