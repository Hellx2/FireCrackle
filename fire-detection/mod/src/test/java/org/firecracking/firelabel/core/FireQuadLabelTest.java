package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import org.junit.jupiter.api.Test;

class FireQuadLabelTest {

    private static final double[][] CORNERS = {{10, 20}, {110, 20}, {110, 120}, {10, 120}};
    private static final double[][] UV = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};

    private static FireQuadLabel quad(int sheet, boolean mirrored, int row, double[][] c, double[][] uv, int vis, int opaque) {
        return new FireQuadLabel(sheet, mirrored, row, c, uv, vis, opaque, 0.0);
    }

    // ---------- basics ----------

    @Test
    void keepsItsFieldsAndDerivesVis() {
        FireQuadLabel q = new FireQuadLabel(1, true, 17, CORNERS, UV, 64, 128, 0.25);
        assertEquals(1, q.sheet);
        assertTrue(q.mirrored);
        assertEquals(17, q.row);
        assertEquals(0.5, q.vis(), 1e-12);
        assertEquals(0.25, q.veil, 1e-12);
        assertEquals(110.0, q.corner(1)[0], 0);
        assertEquals(120.0, q.corner(2)[1], 0);
        assertEquals(0.0, q.uv(1)[0], 0);
        assertEquals(1.0, q.uv(0)[1], 0);
    }

    @Test
    void cornersMayBeNullWhenAPointIsBehindTheCamera() {
        FireQuadLabel q = quad(0, false, 3, null, UV, 5, 10);
        assertFalse(q.hasCorners());
        assertNull(q.corner(0));
        assertTrue(new FireQuadLabel(0, false, 3, CORNERS, UV, 5, 10, 0.0).hasCorners());
    }

    @Test
    void inputsAreCopiedSoLaterChangesCannotLeakIn() {
        double[][] c = {{1, 2}, {3, 4}, {5, 6}, {7, 8}};
        FireQuadLabel q = quad(0, false, 0, c, UV, 1, 2);
        c[0][0] = 999;
        assertEquals(1.0, q.corner(0)[0], 0);
        q.corner(0)[0] = 555; // a returned copy cannot change the label either
        assertEquals(1.0, q.corner(0)[0], 0);
        q.uv(0)[0] = 555;
        assertEquals(1.0, q.uv(0)[0], 0);
    }

    // ---------- failures ----------

    @Test
    void badSheetOrRowIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> quad(2, false, 0, CORNERS, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(-1, false, 0, CORNERS, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, -1, CORNERS, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 32, CORNERS, UV, 1, 2));
    }

    @Test
    void wrongShapedCornersOrUvAreRejected() {
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, new double[][] {{1, 2}, {3, 4}, {5, 6}}, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, new double[][] {{1}, {3, 4}, {5, 6}, {7, 8}}, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, CORNERS, null, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, CORNERS, new double[][] {{0, 0}, {1, 1}}, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, new double[][] {null, {3, 4}, {5, 6}, {7, 8}}, UV, 1, 2));
    }

    @Test
    void nonFiniteNumbersAreRejected() {
        double[][] nan = {{Double.NaN, 2}, {3, 4}, {5, 6}, {7, 8}};
        double[][] inf = {{1, 2}, {3, Double.POSITIVE_INFINITY}, {5, 6}, {7, 8}};
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, nan, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, inf, UV, 1, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, CORNERS, nan, 1, 2));
    }

    @Test
    void visibleTexelsCannotExceedOpaqueAndAtLeastOneMustBeVisible() {
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, CORNERS, UV, 3, 2));
        assertThrows(IllegalArgumentException.class, () -> quad(0, false, 0, CORNERS, UV, 0, 2));
        assertThrows(IllegalArgumentException.class, () -> new FireQuadLabel(0, false, 0, CORNERS, UV, 1, 2, -0.1));
        assertThrows(IllegalArgumentException.class, () -> new FireQuadLabel(0, false, 0, CORNERS, UV, 1, 2, 1.1));
        assertThrows(IllegalArgumentException.class, () -> new FireQuadLabel(0, false, 0, CORNERS, UV, 1, 2, Double.NaN));
    }

    // ---------- on a source ----------

    private static FirePart part() {
        BBox b = new BBox();
        b.add(0, 0);
        b.add(10, 10);
        return new FirePart(0, false, 3, b, 4, 8);
    }

    @Test
    void aSourceKeepsItsQuadsInOrderAndTheyAreImmutable() {
        FireQuadLabel a = quad(0, false, 3, CORNERS, UV, 4, 8);
        FireQuadLabel b = quad(1, true, 9, CORNERS, UV, 2, 8);
        List<FireQuadLabel> list = new ArrayList<FireQuadLabel>(Arrays.asList(a, b));
        FireSourceLabel l = FireSourceLabel.block(1, 2, 3, 4.0, Collections.singletonList(part()), list);
        list.clear(); // changing the caller's list must not matter
        assertEquals(2, l.quads().size());
        assertSame(a, l.quads().get(0));
        assertSame(b, l.quads().get(1));
        assertThrows(UnsupportedOperationException.class, () -> l.quads().add(a));
    }

    @Test
    void sourcesWithoutQuadsHaveAnEmptyList() {
        assertTrue(FireSourceLabel.block(1, 2, 3, 4.0, Collections.singletonList(part())).quads().isEmpty());
        assertTrue(FireSourceLabel.entity(7, "minecraft:cow", 2.0, Collections.singletonList(part())).quads().isEmpty());
        assertTrue(FireSourceLabel.overlay(Collections.singletonList(part())).quads().isEmpty());
    }

    @Test
    void allThreeSourceKindsAcceptQuads() {
        FireQuadLabel q = quad(1, false, 0, CORNERS, UV, 1, 2);
        List<FireQuadLabel> one = Collections.singletonList(q);
        assertEquals(1, FireSourceLabel.block(0, 0, 0, 1.0, Collections.singletonList(part()), one).quads().size());
        assertEquals(1, FireSourceLabel.entity(7, "minecraft:cow", 2.0, Collections.singletonList(part()), one).quads().size());
        assertEquals(1, FireSourceLabel.overlay(Collections.singletonList(part()), one).quads().size());
    }

    @Test
    void aNullQuadListOrANullEntryIsRejected() {
        assertThrows(IllegalArgumentException.class,
                () -> FireSourceLabel.block(0, 0, 0, 1.0, Collections.singletonList(part()), null));
        assertThrows(IllegalArgumentException.class,
                () -> FireSourceLabel.block(0, 0, 0, 1.0, Collections.singletonList(part()), Collections.<FireQuadLabel>singletonList(null)));
    }
}
