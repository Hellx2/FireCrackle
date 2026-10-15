package org.firecracking.firelabel.core;

/** Converts the evaluated full texture plane to the existing v4 log schema. */
public final class FireQuadProjection {
    private FireQuadProjection() {}

    /** Invisible quads are omitted; off-screen corners are deliberately NOT clamped. */
    public static FireQuadLabel capture(int sheet, int row, Quad quad, double[] camera,
            Projector projector, QuadVisibility.Result visibility) {
        if (visibility.visibleTexels == 0) {
            return null;
        }
        double[][] corners = new double[4][];
        double[][] uv = new double[4][];
        boolean projectable = true;
        for (int i = 0; i < 4; i++) {
            double[] v = quad.vertex(i);
            double[] screen = projector.project(v[0]-camera[0], v[1]-camera[1], v[2]-camera[2]);
            if (screen == null) {
                projectable = false;
            } else {
                corners[i] = new double[] {screen[0], screen[1]};
            }
            uv[i] = quad.uv(i);
        }
        return new FireQuadLabel(sheet, quad.isMirroredFrom(camera), row,
                projectable ? corners : null, uv, visibility.visibleTexels,
                visibility.opaqueTexels, Math.min(1.0, visibility.meanVeil()));
    }
}
