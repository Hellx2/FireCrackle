package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import org.junit.jupiter.api.Test;

class FrameRecordTest {

    private static BBox box(double x0, double y0, double x1, double y1) {
        BBox b = new BBox();
        b.add(x0, y0);
        b.add(x1, y1);
        return b;
    }

    private static FirePart part(int sheet, boolean mirrored, int row, BBox box, int visible, int opaque) {
        return new FirePart(sheet, mirrored, row, box, visible, opaque);
    }

    private static FireSourceLabel block(int x, int sheet, boolean mirrored, int row, BBox box) {
        return FireSourceLabel.block(x, 64, -3, 3.25, Collections.singletonList(part(sheet, mirrored, row, box, 128, 256)));
    }

    private static final List<FireSourceLabel> NONE = Collections.<FireSourceLabel>emptyList();

    // ---------- basics ----------

    @Test
    void noFireFrame() {
        FrameRecord r = new FrameRecord(1, 2L, 3, 4, 2560, 1369, NONE);
        assertFalse(r.fire());
        assertEquals("{\"f\":1,\"tick\":2,\"f0\":3,\"f1\":4,\"w\":2560,\"h\":1369,\"fire\":false,\"fires\":[]}", LabelLog.format(r));
    }

    @Test
    void blockSourceHasTheExactSchema() {
        FireSourceLabel b = block(1, 0, false, 17, box(687.5, 337.5, 912.5, 562.5));
        FrameRecord r = new FrameRecord(5, 120L, 17, 1, 2560, 1369, Collections.singletonList(b));
        assertTrue(r.fire());
        assertEquals(
                "{\"f\":5,\"tick\":120,\"f0\":17,\"f1\":1,\"w\":2560,\"h\":1369,\"fire\":true,\"fires\":["
                        + "{\"src\":\"block\",\"pos\":[1,64,-3],\"box\":[687.5,337.5,912.5,562.5],\"vis\":0.500,\"veil\":0.000,\"dist\":3.25,"
                        + "\"parts\":[{\"sheet\":0,\"mirror\":false,\"row\":17,\"box\":[687.5,337.5,912.5,562.5],\"vis\":0.500,\"veil\":0.000}]}"
                        + "]}",
                LabelLog.format(r));
    }

    @Test
    void thereIsNoFrameLevelBoxAnyMore() {
        FireSourceLabel b = block(1, 0, false, 17, box(10, 20, 30, 40));
        String json = LabelLog.format(new FrameRecord(1, 1L, 17, 1, 2560, 1369, Collections.singletonList(b)));
        assertTrue(json.indexOf("\"box\"") > json.indexOf("\"fires\""), "box only appears inside fires: " + json);
        assertFalse(json.contains("\"fire\":true,\"box\""), json);
    }

    @Test
    void aBlockWithSeveralVariantsIsOneEntryWithOneBoxAndSeveralParts() {
        FireSourceLabel b = FireSourceLabel.block(7, 70, 7, 2.0, Arrays.asList(
                part(0, false, 17, box(100, 100, 200, 200), 10, 20),
                part(1, true, 1, box(150, 50, 400, 180), 30, 40)));
        FrameRecord r = new FrameRecord(9, 9L, 17, 1, 2560, 1369, Collections.singletonList(b));
        String json = LabelLog.format(r);
        assertEquals(1, json.split("\"src\":\"block\"", -1).length - 1, "one entry for the one block: " + json);
        assertTrue(json.contains("\"pos\":[7,70,7],\"box\":[100.0,50.0,400.0,200.0],\"vis\":0.667,\"veil\":0.000,\"dist\":2.00"), json);
        assertTrue(json.contains("\"parts\":[{\"sheet\":0,\"mirror\":false,\"row\":17,\"box\":[100.0,100.0,200.0,200.0],\"vis\":0.500,\"veil\":0.000},"
                + "{\"sheet\":1,\"mirror\":true,\"row\":1,\"box\":[150.0,50.0,400.0,180.0],\"vis\":0.750,\"veil\":0.000}]"), json);
    }

    @Test
    void entitySourceHasTheExactSchema() {
        FireSourceLabel e = FireSourceLabel.entity(42, "minecraft:zombie", 2.5,
                Collections.singletonList(part(1, true, 5, box(10, 20, 30, 40), 4, 4)));
        String json = LabelLog.format(new FrameRecord(1, 1L, 3, 5, 2560, 1369, Collections.singletonList(e)));
        assertTrue(json.contains(
                "{\"src\":\"entity\",\"id\":42,\"type\":\"minecraft:zombie\",\"box\":[10.0,20.0,30.0,40.0],"
                        + "\"vis\":1.000,\"veil\":0.000,\"dist\":2.50,\"parts\":[{\"sheet\":1,\"mirror\":true,\"row\":5,"
                        + "\"box\":[10.0,20.0,30.0,40.0],\"vis\":1.000,\"veil\":0.000}]}"), json);
    }

    @Test
    void overlaySourceHasTheExactSchema() {
        FireSourceLabel o = FireSourceLabel.overlay(
                Collections.singletonList(part(1, true, 5, box(0, 600, 1600, 900), 22, 100)));
        String json = LabelLog.format(new FrameRecord(1, 1L, 3, 5, 2560, 1369, Collections.singletonList(o)));
        assertTrue(json.contains(
                "{\"src\":\"overlay\",\"box\":[0.0,600.0,1600.0,900.0],\"vis\":0.220,\"veil\":0.000,\"dist\":0.00,\"parts\":["
                        + "{\"sheet\":1,\"mirror\":true,\"row\":5,\"box\":[0.0,600.0,1600.0,900.0],\"vis\":0.220,\"veil\":0.000}]}"), json);
    }

    @Test
    void sourcesOfAllKindsKeepTheirOrder() {
        List<FirePart> p = Collections.singletonList(part(0, false, 3, box(1, 1, 2, 2), 1, 1));
        FrameRecord r = new FrameRecord(9, 9L, 17, 1, 2560, 1369, Arrays.asList(
                FireSourceLabel.block(1, 2, 3, 1.0, p),
                FireSourceLabel.entity(7, "minecraft:cow", 4.0, p),
                FireSourceLabel.overlay(p)));
        String json = LabelLog.format(r);
        assertTrue(json.indexOf("\"src\":\"block\"") < json.indexOf("\"src\":\"entity\""));
        assertTrue(json.indexOf("\"src\":\"entity\"") < json.indexOf("\"src\":\"overlay\""));
    }

    @Test
    void theFramebufferSizeIsInEveryRowSoBoxesCanBeScaledToAnyVideoSize() {
        String none = LabelLog.format(new FrameRecord(1, 1L, 0, 0, 2560, 1369, NONE));
        assertTrue(none.contains("\"w\":2560,\"h\":1369"), none);
        String small = LabelLog.format(new FrameRecord(1, 1L, 0, 0, 1280, 720, NONE));
        assertTrue(small.contains("\"w\":1280,\"h\":720"), small);
        // right after the sprite rows, before the fire data
        assertTrue(small.indexOf("\"f1\"") < small.indexOf("\"w\"") && small.indexOf("\"w\"") < small.indexOf("\"fire\""));
    }

    @Test
    void anInvalidFramebufferSizeIsRejectedNotDefaulted() {
        assertThrows(IllegalArgumentException.class, () -> new FrameRecord(1, 1L, 0, 0, 0, 720, NONE));
        assertThrows(IllegalArgumentException.class, () -> new FrameRecord(1, 1L, 0, 0, 1280, 0, NONE));
        assertThrows(IllegalArgumentException.class, () -> new FrameRecord(1, 1L, 0, 0, -1, 720, NONE));
    }

    @Test
    void unavailableSpriteRowsAreMinusOne() {
        assertTrue(LabelLog.format(new FrameRecord(7, 9L, -1, -1, 2560, 1369, NONE)).contains("\"f0\":-1,\"f1\":-1"));
    }

    // ---------- failures ----------

    @Test
    void jsonNumbersIgnoreTheSystemLocale() {
        Locale old = Locale.getDefault();
        try {
            Locale.setDefault(Locale.GERMANY);
            FireSourceLabel b = block(1, 0, false, 17, box(687.5, 337.5, 912.5, 562.5));
            String json = LabelLog.format(new FrameRecord(1, 1L, 17, 1, 2560, 1369, Collections.singletonList(b)));
            assertTrue(json.contains("\"dist\":3.25"), json);
            assertTrue(json.contains("687.5"), json);
            assertTrue(json.contains("\"vis\":0.500"), json);
            assertFalse(json.contains("3,25") || json.contains("0,500"), json);
        } finally {
            Locale.setDefault(old);
        }
    }

    @Test
    void boxValuesAreRoundedToOneDecimalAndVisToThree() {
        FireSourceLabel b = FireSourceLabel.block(0, 0, 0, 2.0, Collections.singletonList(
                part(1, false, 5, box(1.04, 2.06, 3.14, 4.96), 12345, 100000)));
        String json = LabelLog.format(new FrameRecord(1, 1L, 5, 5, 2560, 1369, Collections.singletonList(b)));
        assertTrue(json.contains("\"box\":[1.0,2.1,3.1,5.0]"), json);
        assertTrue(json.contains("\"vis\":0.123"), json);
    }

    @Test
    void entityTypeIsJsonEscaped() {
        FireSourceLabel e = FireSourceLabel.entity(1, "a\"b\\c\nd", 1.0,
                Collections.singletonList(part(0, false, 0, box(0, 0, 1, 1), 1, 1)));
        String json = LabelLog.format(new FrameRecord(1, 1L, 0, 0, 2560, 1369, Collections.singletonList(e)));
        assertTrue(json.contains("\"type\":\"a\\\"b\\\\c\\nd\""), json);
    }

    @Test
    void recordCopiesItsSourceList() {
        List<FireSourceLabel> list = new ArrayList<>();
        list.add(block(1, 0, false, 17, box(1, 1, 2, 2)));
        FrameRecord r = new FrameRecord(1, 1L, 17, 1, 2560, 1369, list);
        list.clear();
        assertTrue(r.fire(), "later changes to the caller's list must not change the record");
        assertThrows(UnsupportedOperationException.class, () -> r.fires().add(block(2, 0, false, 0, box(0, 0, 1, 1))));
    }

    @Test
    void theVeilIsInEveryPartAndSource() {
        FireSourceLabel b = FireSourceLabel.block(1, 64, -3, 3.25, Arrays.asList(
                new FirePart(0, false, 17, box(100, 100, 200, 200), 10, 20, 0.0),
                new FirePart(1, true, 1, box(150, 50, 400, 180), 30, 40, 0.8)));
        String json = LabelLog.format(new FrameRecord(1, 1L, 17, 1, 2560, 1369, Collections.singletonList(b)));
        // source: vis = 40/60, veil = (10*0 + 30*0.8) / 40
        assertTrue(json.contains("\"vis\":0.667,\"veil\":0.600,\"dist\":3.25"), json);
        assertTrue(json.contains("\"vis\":0.500,\"veil\":0.000}"), json);
        assertTrue(json.contains("\"vis\":0.750,\"veil\":0.800}"), json);
    }
}
