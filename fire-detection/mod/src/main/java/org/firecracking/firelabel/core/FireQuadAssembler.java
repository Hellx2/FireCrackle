package org.firecracking.firelabel.core;

/**
 * Collects the vertices the game emits one by one for a fire quad ({@code drawFireVertex}: position in view space
 * plus atlas UV) and turns every group of four into a {@link Quad} with sprite-local UV, tagged with which fire
 * sheet it uses. Quads that are not on one of the two fire sprites are dropped, so soul fire and other
 * textures never produce a label.
 */
public final class FireQuadAssembler {
    public static final class Assembled {
        public final Quad quad;
        /** 0 = fire_0, 1 = fire_1. */
        public final int sheet;

        Assembled(Quad quad, int sheet) {
            this.quad = quad;
            this.sheet = sheet;
        }
    }

    private final SpriteRect sprite0;
    private final SpriteRect sprite1;
    private final double[][] pos = new double[4][3];
    private final float[][] uv = new float[4][2];
    private int count;

    public FireQuadAssembler(SpriteRect sprite0, SpriteRect sprite1) {
        if (sprite0 == null || sprite1 == null) {
            throw new IllegalArgumentException("both fire sprites are required");
        }
        this.sprite0 = sprite0;
        this.sprite1 = sprite1;
    }

    /** Returns the finished quad when this is the 4th vertex of a group that is valid fire, else null. */
    public Assembled vertex(double x, double y, double z, float u, float v) {
        pos[count][0] = x;
        pos[count][1] = y;
        pos[count][2] = z;
        uv[count][0] = u;
        uv[count][1] = v;
        count++;
        if (count < 4) {
            return null;
        }
        count = 0;

        for (int i = 0; i < 4; i++) {
            if (!Double.isFinite(pos[i][0]) || !Double.isFinite(pos[i][1]) || !Double.isFinite(pos[i][2])) {
                return null;
            }
        }
        int sheet = allInside(sprite0) ? 0 : (allInside(sprite1) ? 1 : -1);
        if (sheet < 0) {
            return null;
        }
        SpriteRect rect = sheet == 0 ? sprite0 : sprite1;
        double[][] local = new double[4][2];
        for (int i = 0; i < 4; i++) {
            local[i][0] = (uv[i][0] - rect.minU) / (double) (rect.maxU - rect.minU);
            local[i][1] = (uv[i][1] - rect.minV) / (double) (rect.maxV - rect.minV);
        }
        return new Assembled(new Quad(pos, local), sheet);
    }

    /** Drops any vertices of an unfinished quad. */
    public void reset() {
        count = 0;
    }

    private boolean allInside(SpriteRect r) {
        for (int i = 0; i < 4; i++) {
            if (!r.contains(uv[i][0], uv[i][1])) {
                return false;
            }
        }
        return true;
    }
}
