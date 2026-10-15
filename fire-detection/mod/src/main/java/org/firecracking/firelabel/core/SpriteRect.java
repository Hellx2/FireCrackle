package org.firecracking.firelabel.core;

/** Where a sprite sits in the texture atlas, in atlas UV coordinates. */
public final class SpriteRect {
    public final float minU;
    public final float maxU;
    public final float minV;
    public final float maxV;

    public SpriteRect(float minU, float maxU, float minV, float maxV) {
        if (!(maxU > minU) || !(maxV > minV)) {
            throw new IllegalArgumentException("empty sprite rectangle");
        }
        this.minU = minU;
        this.maxU = maxU;
        this.minV = minV;
        this.maxV = maxV;
    }

    boolean contains(float u, float v) {
        float eps = 1e-5f;
        return u >= minU - eps && u <= maxU + eps && v >= minV - eps && v <= maxV + eps;
    }
}
