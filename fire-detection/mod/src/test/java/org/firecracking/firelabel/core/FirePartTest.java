package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class FirePartTest {

    private static BBox box(double x0, double y0, double x1, double y1) {
        BBox b = new BBox();
        b.add(x0, y0);
        b.add(x1, y1);
        return b;
    }

    // ---------- basics ----------

    @Test
    void keepsItsFieldsAndDerivesVis() {
        FirePart p = new FirePart(1, true, 17, box(10, 20, 30, 40), 128, 256);
        assertEquals(1, p.sheet);
        assertTrue(p.mirrored);
        assertEquals(17, p.row);
        assertEquals(128, p.visibleTexels);
        assertEquals(256, p.opaqueTexels);
        assertEquals(0.5, p.vis(), 1e-12);
        assertEquals(30, p.box().x1());
    }

    @Test
    void fullyVisiblePartHasVisOne() {
        assertEquals(1.0, new FirePart(0, false, 0, box(0, 0, 1, 1), 40, 40).vis(), 1e-12);
    }

    @Test
    void boxIsCopiedSoLaterChangesCannotLeakIn() {
        BBox shared = box(1, 1, 2, 2);
        FirePart p = new FirePart(0, false, 3, shared, 1, 2);
        shared.add(500, 500);
        assertEquals(2, p.box().x1());
        p.box().add(900, 900); // and the returned box is a copy too
        assertEquals(2, p.box().x1());
    }

    // ---------- failures ----------

    @Test
    void invalidFieldsAreRejected() {
        BBox ok = box(0, 0, 1, 1);
        assertThrows(IllegalArgumentException.class, () -> new FirePart(2, false, 0, ok, 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(-1, false, 0, ok, 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, -1, ok, 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 32, ok, 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 0, null, 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 0, new BBox(), 1, 1));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 0, ok, 0, 5), "needs a visible texel");
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 0, ok, -1, 5));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 0, ok, 6, 5), "visible > opaque");
    }

    // ---------- GUI veil ----------

    @Test
    void theVeilDefaultsToZeroAndIsStoredWhenGiven() {
        assertEquals(0.0, new FirePart(0, false, 3, box(0, 0, 1, 1), 1, 2).veil, 0.0);
        assertEquals(0.75, new FirePart(0, false, 3, box(0, 0, 1, 1), 1, 2, 0.75).veil, 1e-12);
        assertEquals(1.0, new FirePart(0, false, 3, box(0, 0, 1, 1), 1, 2, 1.0).veil, 1e-12);
    }

    @Test
    void anInvalidVeilIsRejected() {
        BBox ok = box(0, 0, 1, 1);
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 3, ok, 1, 2, -0.01));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 3, ok, 1, 2, 1.01));
        assertThrows(IllegalArgumentException.class, () -> new FirePart(0, false, 3, ok, 1, 2, Double.NaN));
    }
}
