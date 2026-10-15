package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

class QuadVisibilityTest {

    private static final int N = 16;
    private static final double[] ORIGIN = {0, 0, 0};

    private static Projector projector() {
        return new Projector(ProjectorTest.perspective(90, 16.0 / 9.0, 0.05, 512), 1600, 900);
    }

    /** Square facing the camera at z = zPlane (camera looks down -z), x in [x0, x1], y in [-1, 1]. */
    private static Quad quad(double x0, double x1, double zPlane, boolean mirrored) {
        double[][] v = {{x0, -1, zPlane}, {x1, -1, zPlane}, {x1, 1, zPlane}, {x0, 1, zPlane}};
        // uv: v = 0 is the TOP of the texture. Vertex order: bottom-left, bottom-right, top-right, top-left.
        double[][] uv = mirrored
                ? new double[][] {{1, 1}, {0, 1}, {0, 0}, {1, 0}}
                : new double[][] {{0, 1}, {1, 1}, {1, 0}, {0, 0}};
        return new Quad(v, uv);
    }

    private static final TexelMask ALL = (tx, ty) -> true;
    private static final TexelMask NONE = (tx, ty) -> false;
    private static final TexelMask LEFT_HALF = (tx, ty) -> tx < N / 2;
    private static final Occluder CLEAR = (cam, target) -> false;

    private static QuadVisibility.Result eval(Quad q, double[] cam, TexelMask m, Occluder o, int macro) {
        return QuadVisibility.evaluate(q, cam, projector(), N, m, o, macro);
    }

    // ---------- basics ----------

    @Test
    void fullyOpaqueQuadProjectsToItsFullRectangle() {
        QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, ALL, CLEAR, 4);
        assertEquals(256, r.opaqueTexels);
        assertEquals(256, r.visibleTexels);
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(337.5, r.box.y0(), 1e-6);
        assertEquals(912.5, r.box.x1(), 1e-6);
        assertEquals(562.5, r.box.y1(), 1e-6);
    }

    @Test
    void onlyOpaqueTexelsShapeTheBox() {
        QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, LEFT_HALF, CLEAR, 4);
        assertEquals(128, r.opaqueTexels);
        assertEquals(128, r.visibleTexels);
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(800.0, r.box.x1(), 1e-6); // right half of the quad is transparent
        assertEquals(337.5, r.box.y0(), 1e-6);
        assertEquals(562.5, r.box.y1(), 1e-6);
    }

    @Test
    void mirroredQuadShowsTheSameTexelsOnTheOtherSide() {
        QuadVisibility.Result r = eval(quad(-1, 1, -4, true), ORIGIN, LEFT_HALF, CLEAR, 4);
        assertEquals(128, r.visibleTexels);
        assertEquals(800.0, r.box.x0(), 1e-6); // texture's left half now sits on the quad's right side
        assertEquals(912.5, r.box.x1(), 1e-6);
    }

    @Test
    void singleTexelTopLeftHasTheRightTinyBox() {
        TexelMask one = (tx, ty) -> tx == 0 && ty == 0;
        QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, one, CLEAR, 1);
        assertEquals(1, r.opaqueTexels);
        assertEquals(1, r.visibleTexels);
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(337.5, r.box.y0(), 1e-6);
        assertEquals(701.5625, r.box.x1(), 1e-6);
        assertEquals(351.5625, r.box.y1(), 1e-6);
    }

    @Test
    void cameraPositionIsSubtractedBeforeProjecting() {
        double[] cam = {10, 0, 0};
        QuadVisibility.Result r = eval(quad(9, 11, -4, false), cam, ALL, CLEAR, 4);
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(912.5, r.box.x1(), 1e-6);
    }

    // ---------- occlusion ----------

    @Test
    void occludedHalfIsDroppedFromBoxAndVisibleCount() {
        Occluder wallOnTheRight = (cam, target) -> target[0] > 0;
        for (int macro : new int[] {1, 4}) {
            QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, ALL, wallOnTheRight, macro);
            assertEquals(256, r.opaqueTexels, "macro " + macro);
            assertEquals(128, r.visibleTexels, "macro " + macro);
            assertEquals(687.5, r.box.x0(), 1e-6);
            assertEquals(800.0, r.box.x1(), 1e-6);
        }
    }

    @Test
    void fullyOccludedQuadHasNoBox() {
        QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, ALL, (c, t) -> true, 4);
        assertEquals(256, r.opaqueTexels);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void occluderGetsWorldCoordinatesAndIsCachedPerMacroCell() {
        List<double[]> targets = new ArrayList<>();
        List<double[]> cams = new ArrayList<>();
        Occluder recorder = (cam, target) -> {
            cams.add(cam.clone());
            targets.add(target.clone());
            return false;
        };
        double[] cam = {10, 0, 0};
        eval(quad(9, 11, -4, false), cam, ALL, recorder, 4);
        assertEquals(16, targets.size(), "16x16 texels in 4x4 macro cells -> 16 rays, not 256");
        for (double[] t : targets) {
            assertTrue(t[0] > 9 && t[0] < 11, "target is a world-space point on the quad");
            assertEquals(-4.0, t[2], 1e-9);
        }
        for (double[] c : cams) assertArrayEquals(cam, c);
    }

    @Test
    void fineMacroSizeCastsOneRayPerTexel() {
        int[] count = {0};
        eval(quad(-1, 1, -4, false), ORIGIN, ALL, (c, t) -> { count[0]++; return false; }, 1);
        assertEquals(256, count[0]);
    }

    @Test
    void transparentTexelsNeverCastRays() {
        int[] count = {0};
        QuadVisibility.Result r = eval(quad(-1, 1, -4, false), ORIGIN, NONE, (c, t) -> { count[0]++; return false; }, 4);
        assertEquals(0, count[0]);
        assertEquals(0, r.opaqueTexels);
        assertTrue(r.box.isEmpty());
    }

    // ---------- screen limits ----------

    @Test
    void quadBehindTheCameraIsNotVisible() {
        QuadVisibility.Result r = eval(quad(-1, 1, 4, false), ORIGIN, ALL, CLEAR, 4);
        assertEquals(256, r.opaqueTexels);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void quadPartlyOffScreenIsClampedAndFullyOffTexelsAreDropped() {
        // ndcX = x / 7.111 at z = -4, so x in [6, 8.5] crosses the right edge (x = 7.111)
        QuadVisibility.Result r = eval(quad(6, 8.5, -4, false), ORIGIN, ALL, CLEAR, 4);
        assertEquals(256, r.opaqueTexels);
        assertTrue(r.visibleTexels > 0 && r.visibleTexels < 256);
        assertEquals(1475.0, r.box.x0(), 1e-6);
        assertEquals(1600.0, r.box.x1(), 1e-6);
    }

    @Test
    void quadFullyOffScreenIsNotVisible() {
        QuadVisibility.Result r = eval(quad(20, 22, -4, false), ORIGIN, ALL, CLEAR, 4);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void boxNeverLeavesTheScreen() {
        Quad q = quad(-30, 30, -2, false); // much bigger than the view
        QuadVisibility.Result r = eval(q, ORIGIN, ALL, CLEAR, 4);
        assertTrue(r.box.x0() >= 0 && r.box.y0() >= 0 && r.box.x1() <= 1600 && r.box.y1() <= 900);
    }

    // ---------- back-face culling (fire is drawn in a culling layer) ----------

    @Test
    void backFacingQuadIsNotVisible() {
        // same plane and corners, but wound the other way round: it faces away from the camera
        double[][] v = {{1, -1, -4}, {-1, -1, -4}, {-1, 1, -4}, {1, 1, -4}};
        double[][] uv = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};
        QuadVisibility.Result r = eval(new Quad(v, uv), ORIGIN, ALL, CLEAR, 4);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void backFacingQuadCastsNoRays() {
        double[][] v = {{1, -1, -4}, {-1, -1, -4}, {-1, 1, -4}, {1, 1, -4}};
        double[][] uv = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};
        int[] count = {0};
        eval(new Quad(v, uv), ORIGIN, ALL, (c, t) -> { count[0]++; return false; }, 4);
        assertEquals(0, count[0]);
    }

    // ---------- culling can be switched off (entity fire and the overlay face the camera by construction) ----------

    @Test
    void withCullingOffABackWoundQuadIsStillVisible() {
        double[][] v = {{1, -1, -4}, {-1, -1, -4}, {-1, 1, -4}, {1, 1, -4}};
        double[][] uv = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};
        Quad q = new Quad(v, uv);
        QuadVisibility.Result off = QuadVisibility.evaluate(q, ORIGIN, projector(), N, ALL, CLEAR, 4, false);
        assertEquals(256, off.visibleTexels);
        assertEquals(687.5, off.box.x0(), 1e-6);
        assertEquals(912.5, off.box.x1(), 1e-6);
        QuadVisibility.Result on = QuadVisibility.evaluate(q, ORIGIN, projector(), N, ALL, CLEAR, 4, true);
        assertEquals(0, on.visibleTexels);
    }

    @Test
    void theSevenArgumentFormStillCulls() {
        double[][] v = {{1, -1, -4}, {-1, -1, -4}, {-1, 1, -4}, {1, 1, -4}};
        double[][] uv = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};
        assertEquals(0, eval(new Quad(v, uv), ORIGIN, ALL, CLEAR, 4).visibleTexels);
    }

    // ---------- GUI: only opaque GUI hides fire; a translucent veil is kept and reported ----------

    private static QuadVisibility.Result withGui(Quad q, int texels, int macro, ScreenMask gui) {
        return QuadVisibility.evaluate(q, ORIGIN, projector(), texels, ALL, CLEAR, macro, true, gui);
    }

    @Test
    void opaqueGuiHidesTheCoveredTextureCells() {
        // the quad spans screen x 687.5..912.5; an opaque GUI covers everything from x = 800 on, the right half
        QuadVisibility.Result r = withGui(quad(-1, 1, -4, false), N, 4, (x, y) -> x >= 800 ? 1.0 : 0.0);
        assertEquals(256, r.opaqueTexels, "vis is still measured against all opaque texels");
        assertEquals(128, r.visibleTexels);
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(800.0, r.box.x1(), 1e-6);
        assertEquals(0.0, r.meanVeil(), 1e-12);
    }

    @Test
    void aTranslucentVeilKeepsTheFireAndIsReportedAsData() {
        QuadVisibility.Result r = withGui(quad(-1, 1, -4, false), N, 4, (x, y) -> 0.75);
        assertEquals(256, r.visibleTexels, "a 75% dark veil does not hide the fire");
        assertEquals(687.5, r.box.x0(), 1e-6);
        assertEquals(912.5, r.box.x1(), 1e-6);
        assertEquals(0.75, r.meanVeil(), 1e-9);
    }

    @Test
    void theVeilIsAveragedOverTheTexelsThatRemainVisible() {
        // left half under a 50% veil, right half under an opaque panel: only the left half is visible
        QuadVisibility.Result r = withGui(quad(-1, 1, -4, false), N, 4, (x, y) -> x >= 800 ? 1.0 : 0.5);
        assertEquals(128, r.visibleTexels);
        assertEquals(0.5, r.meanVeil(), 1e-9);
    }

    @Test
    void onlyNearlyOpaqueAlphaCountsAsHiding() {
        Quad q = quad(-1, 1, -4, false);
        assertEquals(0, withGui(q, N, 4, (x, y) -> 0.99).visibleTexels, "0.99 is opaque enough to hide");
        QuadVisibility.Result r = withGui(q, N, 4, (x, y) -> 0.97);
        assertEquals(256, r.visibleTexels, "0.97 is a veil, not a wall");
        assertEquals(0.97, r.meanVeil(), 1e-9);
    }

    @Test
    void aFullyOpaqueGuiHidesEverythingAndCastsNoRays() {
        int[] rays = {0};
        QuadVisibility.Result r = QuadVisibility.evaluate(quad(-1, 1, -4, false), ORIGIN, projector(), N, ALL,
                (c, t) -> { rays[0]++; return false; }, 4, true, (x, y) -> 1.0);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
        assertEquals(0.0, r.meanVeil(), 1e-12);
        assertEquals(0, rays[0], "texels under an opaque GUI must not cost a ray cast");
    }

    @Test
    void aCellThatIsMostlyOpaqueIsHiddenAndOneThatIsMostlyFreeIsKept() {
        // one single texel spanning screen x 687.5..912.5; its 3 sample columns are at x = 725, 800, 875
        Quad q = quad(-1, 1, -4, false);
        assertEquals(0, withGui(q, 1, 1, (x, y) -> x >= 760 ? 1.0 : 0.0).visibleTexels, "2 of 3 columns opaque: hidden");
        QuadVisibility.Result kept = withGui(q, 1, 1, (x, y) -> x >= 840 ? 1.0 : 0.0);
        assertEquals(1, kept.visibleTexels, "1 of 3 columns opaque: kept");
        assertEquals(1.0 / 3.0, kept.meanVeil(), 1e-9, "the veil is the mean alpha of the nine samples");
        assertEquals(1, withGui(q, 1, 1, (x, y) -> 0.0).visibleTexels);
    }

    @Test
    void anAbsentMaskMeansNoGui() {
        QuadVisibility.Result with = QuadVisibility.evaluate(quad(-1, 1, -4, false), ORIGIN, projector(), N, ALL, CLEAR, 4, true, null);
        QuadVisibility.Result without = eval(quad(-1, 1, -4, false), ORIGIN, ALL, CLEAR, 4);
        assertEquals(without.visibleTexels, with.visibleTexels);
        assertEquals(without.box.x0(), with.box.x0(), 1e-9);
        assertEquals(without.box.x1(), with.box.x1(), 1e-9);
        assertEquals(0.0, with.meanVeil(), 1e-12);
    }

    @Test
    void theMaskIsAskedInTopLeftScreenPixels() {
        double[] seen = {Double.MAX_VALUE, Double.MAX_VALUE, -1, -1};
        withGui(quad(-1, 1, -4, false), N, 4, (x, y) -> {
            seen[0] = Math.min(seen[0], x);
            seen[1] = Math.min(seen[1], y);
            seen[2] = Math.max(seen[2], x);
            seen[3] = Math.max(seen[3], y);
            return 0.0;
        });
        assertTrue(seen[0] >= 687.5 - 1e-6 && seen[2] <= 912.5 + 1e-6, "x range " + seen[0] + ".." + seen[2]);
        assertTrue(seen[1] >= 337.5 - 1e-6 && seen[3] <= 562.5 + 1e-6, "y range " + seen[1] + ".." + seen[3]);
    }

    @Test
    void theGuiAlsoHidesTextureCellsThatAreOnlyPartlyOnScreen() {
        // quad half off the right edge: the check works on the on-screen part
        Quad q = quad(6, 8.5, -4, false);
        QuadVisibility.Result free = withGui(q, N, 4, (x, y) -> 0.0);
        QuadVisibility.Result covered = withGui(q, N, 4, (x, y) -> x >= 1550 ? 1.0 : 0.0);
        assertTrue(covered.visibleTexels < free.visibleTexels);
        assertTrue(covered.box.x1() <= 1600.0);
    }

    // ---------- degenerate input ----------

    @Test
    void degenerateUvGivesAnEmptyResultWithoutExceptions() {
        double[][] v = {{-1, -1, -4}, {1, -1, -4}, {1, 1, -4}, {-1, 1, -4}};
        double[][] uv = {{0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}, {0.5, 0.5}};
        QuadVisibility.Result r = eval(new Quad(v, uv), ORIGIN, ALL, CLEAR, 4);
        assertEquals(0, r.opaqueTexels);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void zeroAreaQuadIsNeverVisible() {
        double[][] v = {{0, 0, -4}, {0, 0, -4}, {0, 0, -4}, {0, 0, -4}};
        double[][] uv = {{0, 1}, {1, 1}, {1, 0}, {0, 0}};
        QuadVisibility.Result r = eval(new Quad(v, uv), ORIGIN, ALL, CLEAR, 4);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void edgeOnQuadIsNeverVisible() {
        // a quad seen exactly edge-on projects to a line: zero area, so not visible
        double[][] v = {{0, -1, -2}, {0, -1, -6}, {0, 1, -6}, {0, 1, -2}};
        double[][] uv = {{0, 1}, {1, 1}, {1, 0}, {0, 0}};
        QuadVisibility.Result r = eval(new Quad(v, uv), ORIGIN, ALL, CLEAR, 4);
        assertEquals(0, r.visibleTexels);
        assertTrue(r.box.isEmpty());
    }

    @Test
    void badArgumentsAreRejected() {
        Quad q = quad(-1, 1, -4, false);
        assertThrows(IllegalArgumentException.class, () -> eval(q, ORIGIN, ALL, CLEAR, 0));
        assertThrows(IllegalArgumentException.class,
                () -> QuadVisibility.evaluate(q, ORIGIN, projector(), 0, ALL, CLEAR, 4));
        assertThrows(IllegalArgumentException.class, () -> new Quad(new double[3][3], new double[4][2]));
        assertThrows(IllegalArgumentException.class, () -> new Quad(new double[4][3], new double[4][3]));
    }
}
