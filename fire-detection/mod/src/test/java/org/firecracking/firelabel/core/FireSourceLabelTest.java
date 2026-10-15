package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import org.junit.jupiter.api.Test;

class FireSourceLabelTest {

    private static BBox box(double x0, double y0, double x1, double y1) {
        BBox b = new BBox();
        b.add(x0, y0);
        b.add(x1, y1);
        return b;
    }

    private static FirePart part(int sheet, boolean mirrored, int row, BBox box, int visible, int opaque) {
        return new FirePart(sheet, mirrored, row, box, visible, opaque);
    }

    // ---------- basics ----------

    @Test
    void oneBoxPerSourceIsTheUnionOfItsParts() {
        FireSourceLabel l = FireSourceLabel.block(1, 64, -3, 3.25, Arrays.asList(
                part(0, false, 17, box(100, 100, 200, 200), 10, 20),
                part(1, true, 1, box(150, 50, 400, 180), 30, 40)));
        BBox b = l.box();
        assertEquals(100, b.x0());
        assertEquals(50, b.y0());
        assertEquals(400, b.x1());
        assertEquals(200, b.y1());
    }

    @Test
    void sourceVisIsVisibleOverOpaqueAcrossAllParts() {
        FireSourceLabel l = FireSourceLabel.block(0, 0, 0, 1.0, Arrays.asList(
                part(0, false, 0, box(0, 0, 1, 1), 10, 20),
                part(1, true, 0, box(0, 0, 1, 1), 30, 40)));
        assertEquals(40.0 / 60.0, l.vis(), 1e-12);
    }

    @Test
    void partsKeepTheirOrderAndAreReadOnly() {
        FirePart a = part(0, false, 17, box(0, 0, 1, 1), 1, 2);
        FirePart b = part(1, false, 1, box(0, 0, 1, 1), 1, 2);
        FireSourceLabel l = FireSourceLabel.block(0, 0, 0, 1.0, Arrays.asList(a, b));
        assertEquals(2, l.parts().size());
        assertSame(a, l.parts().get(0));
        assertSame(b, l.parts().get(1));
        assertThrows(UnsupportedOperationException.class, () -> l.parts().add(a));
    }

    @Test
    void blockEntityAndOverlayFactories() {
        List<FirePart> one = Collections.singletonList(part(1, true, 5, box(10, 20, 30, 40), 5, 5));
        FireSourceLabel blk = FireSourceLabel.block(1, 2, 3, 4.5, one);
        assertEquals(FireSourceLabel.Source.BLOCK, blk.source);
        assertEquals(1, blk.x);
        assertEquals(2, blk.y);
        assertEquals(3, blk.z);
        assertEquals(4.5, blk.dist, 1e-12);

        FireSourceLabel ent = FireSourceLabel.entity(42, "minecraft:cow", 2.5, one);
        assertEquals(FireSourceLabel.Source.ENTITY, ent.source);
        assertEquals(42, ent.entityId);
        assertEquals("minecraft:cow", ent.entityType);

        FireSourceLabel ov = FireSourceLabel.overlay(one);
        assertEquals(FireSourceLabel.Source.OVERLAY, ov.source);
        assertEquals(0.0, ov.dist, 1e-12);
    }

    // ---------- failures ----------

    @Test
    void aSourceNeedsAtLeastOnePart() {
        assertThrows(IllegalArgumentException.class,
                () -> FireSourceLabel.block(0, 0, 0, 1.0, Collections.<FirePart>emptyList()));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.block(0, 0, 0, 1.0, null));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.overlay(Collections.<FirePart>emptyList()));
    }

    @Test
    void twoPartsOfTheSameSheetAndMirrorAreRejected() {
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.block(0, 0, 0, 1.0, Arrays.asList(
                part(0, false, 3, box(0, 0, 1, 1), 1, 2),
                part(0, false, 3, box(5, 5, 6, 6), 1, 2))));
    }

    @Test
    void sameSheetWithDifferentMirrorIsFine() {
        assertDoesNotThrow(() -> FireSourceLabel.block(0, 0, 0, 1.0, Arrays.asList(
                part(0, false, 3, box(0, 0, 1, 1), 1, 2),
                part(0, true, 3, box(5, 5, 6, 6), 1, 2))));
    }

    @Test
    void invalidDistOrEntityTypeIsRejected() {
        List<FirePart> one = Collections.singletonList(part(0, false, 0, box(0, 0, 1, 1), 1, 1));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.block(0, 0, 0, -1.0, one));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.block(0, 0, 0, Double.NaN, one));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.block(0, 0, 0, Double.POSITIVE_INFINITY, one));
        assertThrows(IllegalArgumentException.class, () -> FireSourceLabel.entity(1, null, 1.0, one));
    }

    @Test
    void labelCopiesItsPartList() {
        List<FirePart> list = new ArrayList<>();
        list.add(part(0, false, 0, box(0, 0, 1, 1), 1, 1));
        FireSourceLabel l = FireSourceLabel.block(0, 0, 0, 1.0, list);
        list.clear();
        assertEquals(1, l.parts().size());
    }

    @Test
    void boxReturnedIsACopy() {
        FireSourceLabel l = FireSourceLabel.block(0, 0, 0, 1.0,
                Collections.singletonList(part(0, false, 0, box(0, 0, 1, 1), 1, 1)));
        l.box().add(999, 999);
        assertEquals(1, l.box().x1());
    }

    // ---------- GUI veil ----------

    @Test
    void theSourceVeilIsTheVisibleTexelWeightedMeanOfItsParts() {
        FireSourceLabel l = FireSourceLabel.block(0, 0, 0, 1.0, Arrays.asList(
                new FirePart(0, false, 3, box(0, 0, 1, 1), 10, 20, 0.0),
                new FirePart(1, true, 3, box(0, 0, 1, 1), 30, 40, 0.8)));
        assertEquals((10 * 0.0 + 30 * 0.8) / 40.0, l.veil(), 1e-12);
    }

    @Test
    void aSourceWithNoVeilHasZero() {
        FireSourceLabel l = FireSourceLabel.overlay(Collections.singletonList(part(1, true, 5, box(0, 0, 1, 1), 5, 5)));
        assertEquals(0.0, l.veil(), 0.0);
    }
}
