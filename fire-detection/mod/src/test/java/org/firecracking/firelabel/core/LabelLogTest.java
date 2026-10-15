package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.io.StringWriter;
import java.io.Writer;
import java.util.Collections;
import org.junit.jupiter.api.Test;

class LabelLogTest {

    /** Tail of a frame row that has no visible fire (schema: see mod/PLAN.md, iteration 2). */
    private static final String NO_FIRE = ",\"w\":2560,\"h\":1369,\"fire\":false,\"fires\":[]}";

    private static final class CountingWriter extends Writer {
        final StringWriter inner = new StringWriter();
        int closes = 0;

        @Override
        public void write(char[] buf, int off, int len) {
            inner.write(buf, off, len);
        }

        @Override
        public void flush() {}

        @Override
        public void close() {
            closes++;
        }
    }

    // ---------- basics ----------

    @Test
    void lineFormatIsExact() {
        assertEquals("{\"f\":5,\"tick\":120,\"f0\":17,\"f1\":17" + NO_FIRE, LabelLog.format(5, 120L, 17, 17, 2560, 1369));
        assertEquals("{\"f\":0,\"tick\":0,\"f0\":0,\"f1\":31" + NO_FIRE, LabelLog.format(0, 0L, 0, 31, 2560, 1369));
    }

    @Test
    void largeTickCountsDoNotOverflow() {
        assertEquals(
                "{\"f\":16777215,\"tick\":9876543210,\"f0\":3,\"f1\":4" + NO_FIRE,
                LabelLog.format(16777215, 9876543210L, 3, 4, 2560, 1369));
    }

    @Test
    void oneLinePerWriteAndContentIsVisibleAfterClose() throws IOException {
        CountingWriter out = new CountingWriter();
        LabelLog log = new LabelLog(out);
        log.write(1, 10L, 4, 4, 2560, 1369);
        log.write(2, 10L, 4, 4, 2560, 1369);
        log.write(3, 11L, 5, 5, 2560, 1369);
        log.close();
        assertEquals(
                "{\"f\":1,\"tick\":10,\"f0\":4,\"f1\":4" + NO_FIRE + "\n"
                        + "{\"f\":2,\"tick\":10,\"f0\":4,\"f1\":4" + NO_FIRE + "\n"
                        + "{\"f\":3,\"tick\":11,\"f0\":5,\"f1\":5" + NO_FIRE + "\n",
                out.inner.toString());
    }

    @Test
    void writesAFullFrameRecord() throws IOException {
        BBox b = new BBox();
        b.add(10, 20);
        b.add(30, 40);
        FrameRecord r = new FrameRecord(8, 99L, 3, 19, 2560, 1369, Collections.singletonList(
                FireSourceLabel.block(1, 2, 3, 5.0, Collections.singletonList(new FirePart(1, true, 19, b, 4, 4)))));
        CountingWriter out = new CountingWriter();
        LabelLog log = new LabelLog(out);
        log.write(r);
        log.close();
        assertEquals(LabelLog.format(r) + "\n", out.inner.toString());
        assertTrue(out.inner.toString().contains("\"fire\":true"));
    }

    // ---------- failures ----------

    @Test
    void unavailableSpriteIsLoggedAsMinusOne() {
        assertEquals("{\"f\":7,\"tick\":9,\"f0\":-1,\"f1\":-1" + NO_FIRE, LabelLog.format(7, 9L, -1, -1, 2560, 1369));
    }

    @Test
    void writingAfterCloseIsAnError() throws IOException {
        LabelLog log = new LabelLog(new CountingWriter());
        log.close();
        assertThrows(IllegalStateException.class, () -> log.write(1, 1L, 1, 1, 2560, 1369));
        assertThrows(IllegalStateException.class, () -> log.write(
                new FrameRecord(1, 1L, 1, 1, 2560, 1369, Collections.<FireSourceLabel>emptyList())));
    }

    @Test
    void closingTwiceClosesTheUnderlyingWriterOnce() throws IOException {
        CountingWriter out = new CountingWriter();
        LabelLog log = new LabelLog(out);
        log.close();
        log.close();
        assertEquals(1, out.closes);
    }

    @Test
    void flushMakesBufferedLinesVisibleWithoutClosing() throws IOException {
        CountingWriter out = new CountingWriter();
        LabelLog log = new LabelLog(out);
        log.write(1, 1L, 1, 1, 2560, 1369);
        log.flush();
        assertEquals("{\"f\":1,\"tick\":1,\"f0\":1,\"f1\":1" + NO_FIRE + "\n", out.inner.toString());
        assertEquals(0, out.closes);
        log.close();
    }

    // ---------- schema v4: quads ----------

    private static final double[][] QC = {{10, 20}, {110, 20}, {110, 120}, {10, 120}};
    private static final double[][] QUV = {{1, 1}, {0, 1}, {0, 0}, {1, 0}};

    private static FrameRecord oneBlock(java.util.List<FireQuadLabel> quads) {
        BBox b = new BBox();
        b.add(10, 20);
        b.add(30, 40);
        return new FrameRecord(8, 99L, 3, 19, 2560, 1369, Collections.singletonList(
                FireSourceLabel.block(1, 2, 3, 5.0, Collections.singletonList(new FirePart(1, true, 19, b, 4, 4)), quads)));
    }

    @Test
    void aSourceWithoutQuadsIsWrittenExactlyAsBefore() {
        String line = LabelLog.format(oneBlock(Collections.<FireQuadLabel>emptyList()));
        assertFalse(line.contains("quads"));
        assertEquals(LabelLog.format(oneBlock(Collections.<FireQuadLabel>emptyList())), line);
    }

    @Test
    void aQuadIsWrittenAfterThePartsWithCornersUvAndVisibility() {
        FireQuadLabel q = new FireQuadLabel(1, true, 19, QC, QUV, 64, 128, 0.25);
        String line = LabelLog.format(oneBlock(Collections.singletonList(q)));
        assertTrue(line.endsWith(
                "],\"quads\":[{\"s\":1,\"m\":1,\"row\":19,"
                        + "\"c\":[10.0,20.0,110.0,20.0,110.0,120.0,10.0,120.0],"
                        + "\"uv\":[1.000,1.000,0.000,1.000,0.000,0.000,1.000,0.000],"
                        + "\"vis\":0.500,\"veil\":0.250}]}]}"),
                line);
    }

    @Test
    void quadsKeepTheirOrderAndAreCommaSeparated() {
        FireQuadLabel a = new FireQuadLabel(0, false, 3, QC, QUV, 1, 2, 0.0);
        FireQuadLabel b = new FireQuadLabel(1, true, 9, QC, QUV, 2, 2, 0.0);
        String line = LabelLog.format(oneBlock(java.util.Arrays.asList(a, b)));
        int ia = line.indexOf("\"row\":3,\"c\"");
        int ib = line.indexOf("\"row\":9,\"c\"");
        assertTrue(ia > 0 && ib > ia, line);
        assertTrue(line.contains("}},{\"s\":1") || line.contains("},{\"s\":1"), line);
    }

    @Test
    void aQuadWithACornerBehindTheCameraWritesNullCorners() {
        FireQuadLabel q = new FireQuadLabel(0, false, 5, null, QUV, 1, 2, 0.0);
        assertTrue(LabelLog.format(oneBlock(Collections.singletonList(q))).contains("\"c\":null,\"uv\":["));
    }

    @Test
    void numbersNeverUseADecimalCommaWhateverTheDefaultLocale() {
        java.util.Locale old = java.util.Locale.getDefault();
        try {
            java.util.Locale.setDefault(java.util.Locale.GERMANY);
            FireQuadLabel q = new FireQuadLabel(0, false, 5, QC, QUV, 1, 3, 0.125);
            String line = LabelLog.format(oneBlock(Collections.singletonList(q)));
            assertTrue(line.contains("\"vis\":0.333,\"veil\":0.125"), line);
            assertTrue(line.contains("[10.0,20.0,"), line);
        } finally {
            java.util.Locale.setDefault(old);
        }
    }

    @Test
    void aLineWithQuadsStaysBalancedJson() {
        FireQuadLabel q = new FireQuadLabel(0, false, 5, QC, QUV, 1, 3, 0.0);
        String line = LabelLog.format(oneBlock(java.util.Arrays.asList(q, q)));
        int curlies = 0;
        int squares = 0;
        for (char c : line.toCharArray()) {
            curlies += c == '{' ? 1 : c == '}' ? -1 : 0;
            squares += c == '[' ? 1 : c == ']' ? -1 : 0;
        }
        assertEquals(0, curlies);
        assertEquals(0, squares);
        assertFalse(line.contains("NaN"));
        assertFalse(line.contains("Infinity"));
    }
}
