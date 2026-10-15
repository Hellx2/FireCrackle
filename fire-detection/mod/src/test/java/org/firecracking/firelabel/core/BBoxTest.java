package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class BBoxTest {
    @Test
    void startsEmpty() {
        assertTrue(new BBox().isEmpty());
    }

    @Test
    void growsToCoverAddedPoints() {
        BBox b = new BBox();
        b.add(10, 20);
        assertFalse(b.isEmpty());
        assertEquals(10, b.x0());
        assertEquals(10, b.x1());
        b.add(5, 30);
        b.add(12, 15);
        assertEquals(5, b.x0());
        assertEquals(12, b.x1());
        assertEquals(15, b.y0());
        assertEquals(30, b.y1());
    }

    @Test
    void unionOfTwoBoxes() {
        BBox a = new BBox();
        a.add(0, 0);
        a.add(10, 10);
        BBox b = new BBox();
        b.add(5, 5);
        b.add(20, 8);
        a.add(b);
        assertEquals(0, a.x0());
        assertEquals(20, a.x1());
        assertEquals(0, a.y0());
        assertEquals(10, a.y1());
    }

    @Test
    void addingAnEmptyBoxChangesNothing() {
        BBox a = new BBox();
        a.add(1, 2);
        a.add(new BBox());
        assertEquals(1, a.x0());
        assertEquals(2, a.y1());
        BBox empty = new BBox();
        empty.add(new BBox());
        assertTrue(empty.isEmpty());
    }

    @Test
    void clampKeepsTheInsidePart() {
        BBox b = new BBox();
        b.add(-50, 10);
        b.add(120, 300);
        BBox c = b.clamped(100, 200);
        assertEquals(0, c.x0());
        assertEquals(100, c.x1());
        assertEquals(10, c.y0());
        assertEquals(200, c.y1());
    }

    @Test
    void clampOfABoxFullyOutsideIsEmpty() {
        BBox b = new BBox();
        b.add(150, 10);
        b.add(200, 20);
        assertTrue(b.clamped(100, 100).isEmpty());
        BBox c = new BBox();
        c.add(-30, -30);
        c.add(-1, -1);
        assertTrue(c.clamped(100, 100).isEmpty());
        assertTrue(new BBox().clamped(100, 100).isEmpty());
    }

    @Test
    void clampDoesNotModifyTheOriginal() {
        BBox b = new BBox();
        b.add(-5, -5);
        b.add(500, 500);
        b.clamped(100, 100);
        assertEquals(-5, b.x0());
        assertEquals(500, b.x1());
    }

    @Test
    void nonFinitePointsAreRejected() {
        BBox b = new BBox();
        assertThrows(IllegalArgumentException.class, () -> b.add(Double.NaN, 1));
        assertThrows(IllegalArgumentException.class, () -> b.add(1, Double.POSITIVE_INFINITY));
        assertTrue(b.isEmpty());
    }
}
