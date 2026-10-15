package org.firecracking.firelabel.core;

/**
 * A textured parallelogram in world space. {@code v} are 4 corners {@code {x, y, z}} going around the quad;
 * {@code uv} are the sprite-local texture coordinates (0..1, v = 0 is the top of the sprite) of each corner.
 * Fire quads are rectangles, so the UV map is affine: P(s, t) = v0 + s (v1 - v0) + t (v3 - v0).
 */
public final class Quad {
    final double[][] v;
    final double[][] uv;

    public Quad(double[][] v, double[][] uv) {
        if (v == null || v.length != 4 || uv == null || uv.length != 4) {
            throw new IllegalArgumentException("a quad needs 4 vertices and 4 uv pairs");
        }
        this.v = new double[4][];
        this.uv = new double[4][];
        for (int i = 0; i < 4; i++) {
            if (v[i] == null || v[i].length != 3) {
                throw new IllegalArgumentException("vertex " + i + " must be {x, y, z}");
            }
            if (uv[i] == null || uv[i].length != 2) {
                throw new IllegalArgumentException("uv " + i + " must be {u, v}");
            }
            this.v[i] = v[i].clone();
            this.uv[i] = uv[i].clone();
        }
    }

    /** A copy of corner {@code i} as {@code {x, y, z}}. */
    public double[] vertex(int i) {
        return v[i].clone();
    }

    /** A copy of corner {@code i}'s sprite-local texture coordinate as {@code {u, v}}. */
    public double[] uv(int i) {
        return uv[i].clone();
    }

    /**
     * True if the texture appears flipped horizontally on screen, as seen from {@code cam}. Unlike
     * {@link #isMirrored()} this does not assume the quad is wound counter-clockwise from the front: it
     * combines the UV orientation with which side of the quad the camera is on. Degenerate quads give false.
     */
    public boolean isMirroredFrom(double[] cam) {
        double a = uv[1][0] - uv[0][0];
        double b = uv[3][0] - uv[0][0];
        double c = uv[1][1] - uv[0][1];
        double d = uv[3][1] - uv[0][1];
        double det = a * d - b * c;
        if (Math.abs(det) < 1e-12) {
            return false;
        }
        double e1x = v[1][0] - v[0][0];
        double e1y = v[1][1] - v[0][1];
        double e1z = v[1][2] - v[0][2];
        double e3x = v[3][0] - v[0][0];
        double e3y = v[3][1] - v[0][1];
        double e3z = v[3][2] - v[0][2];
        double nx = e1y * e3z - e1z * e3y;
        double ny = e1z * e3x - e1x * e3z;
        double nz = e1x * e3y - e1y * e3x;
        if (nx * nx + ny * ny + nz * nz < 1e-24) {
            return false;
        }
        double dot = nx * (cam[0] - v[0][0]) + ny * (cam[1] - v[0][1]) + nz * (cam[2] - v[0][2]);
        boolean uvPositive = det > 0;
        return dot > 0 ? uvPositive : !uvPositive;
    }

    /**
     * True if the texture is flipped horizontally as seen from the front (the {@code *_alt*} fire models).
     * Baked quads are wound counter-clockwise from the front; with that winding an unmirrored texture
     * (v grows downwards) has a negative UV determinant and a mirrored one a positive determinant.
     * Degenerate UV maps count as not mirrored.
     */
    public boolean isMirrored() {
        double a = uv[1][0] - uv[0][0];
        double b = uv[3][0] - uv[0][0];
        double c = uv[1][1] - uv[0][1];
        double d = uv[3][1] - uv[0][1];
        return a * d - b * c > 1e-12;
    }

    /**
     * True if the camera is on the side the quad's counter-clockwise winding faces. Fire is drawn in a
     * back-face-culling layer, so back-facing quads are not visible. Zero-area quads are never front-facing.
     */
    public boolean isFrontFacing(double[] cam) {
        double e1x = v[1][0] - v[0][0];
        double e1y = v[1][1] - v[0][1];
        double e1z = v[1][2] - v[0][2];
        double e3x = v[3][0] - v[0][0];
        double e3y = v[3][1] - v[0][1];
        double e3z = v[3][2] - v[0][2];
        double nx = e1y * e3z - e1z * e3y;
        double ny = e1z * e3x - e1x * e3z;
        double nz = e1x * e3y - e1y * e3x;
        double dot = nx * (cam[0] - v[0][0]) + ny * (cam[1] - v[0][1]) + nz * (cam[2] - v[0][2]);
        return dot > 1e-12;
    }
}
