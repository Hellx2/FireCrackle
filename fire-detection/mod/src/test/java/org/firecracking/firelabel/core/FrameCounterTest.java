package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class FrameCounterTest {
    @Test
    void startsAtZeroAndIncrements() {
        FrameCounter c = new FrameCounter();
        assertEquals(0, c.next());
        assertEquals(1, c.next());
        assertEquals(2, c.next());
    }

    @Test
    void wrapsToZeroAfterTheLargestId() {
        FrameCounter c = new FrameCounter(StripPattern.MAX_ID - 1);
        assertEquals(StripPattern.MAX_ID - 1, c.next());
        assertEquals(StripPattern.MAX_ID, c.next());
        assertEquals(0, c.next());
        assertEquals(1, c.next());
    }

    @Test
    void everyIdFromTheCounterIsEncodable() {
        FrameCounter c = new FrameCounter(StripPattern.MAX_ID - 3);
        for (int i = 0; i < 8; i++) {
            StripPattern.cells(0, c.next()); // must not throw
        }
    }

    @Test
    void startValueOutOfRangeIsRejected() {
        org.junit.jupiter.api.Assertions.assertThrows(
                IllegalArgumentException.class, () -> new FrameCounter(StripPattern.MAX_ID + 1));
        org.junit.jupiter.api.Assertions.assertThrows(
                IllegalArgumentException.class, () -> new FrameCounter(-1));
    }
}
