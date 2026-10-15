package org.firecracking.firelabel.core;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

/** Entity fire arrives vertex by vertex (EntityRenderDispatcher.drawFireVertex); four make a quad. */
class FireQuadAssemblerTest {

    // two sprites in the atlas: fire_0 at u [0.10, 0.20], v [0.30, 0.40]; fire_1 at u [0.50, 0.60], v [0.30, 0.40]
    private static final SpriteRect S0 = new SpriteRect(0.10f, 0.20f, 0.30f, 0.40f);
    private static final SpriteRect S1 = new SpriteRect(0.50f, 0.60f, 0.30f, 0.40f);

    private static FireQuadAssembler assembler() {
        return new FireQuadAssembler(S0, S1);
    }

    /** Feeds one quad (BR, BL, TL, TR like the game) with the given U range, returns what the 4th vertex gave. */
    private static FireQuadAssembler.Assembled feed(FireQuadAssembler a, float uLeft, float uRight, float vTop, float vBottom) {
        assertNull(a.vertex(1, -1, -3, uRight, vBottom));
        assertNull(a.vertex(-1, -1, -3, uLeft, vBottom));
        assertNull(a.vertex(-1, 1, -3, uLeft, vTop));
        return a.vertex(1, 1, -3, uRight, vTop);
    }

    // ---------- basics ----------

    @Test
    void fourVerticesOnFire0MakeASheet0Quad() {
        FireQuadAssembler.Assembled q = feed(assembler(), 0.10f, 0.20f, 0.30f, 0.40f);
        assertNotNull(q);
        assertEquals(0, q.sheet);
        assertArrayEquals(new double[] {1, -1, -3}, q.quad.vertex(0), 1e-9);
        assertArrayEquals(new double[] {1, 1, -3}, q.quad.vertex(3), 1e-9);
    }

    @Test
    void fourVerticesOnFire1MakeASheet1Quad() {
        FireQuadAssembler.Assembled q = feed(assembler(), 0.50f, 0.60f, 0.30f, 0.40f);
        assertNotNull(q);
        assertEquals(1, q.sheet);
    }

    @Test
    void uvIsNormalisedToTheSpriteRectangle() {
        FireQuadAssembler.Assembled q = feed(assembler(), 0.10f, 0.20f, 0.30f, 0.40f);
        // vertex 0 = bottom-right: u = max, v = max -> (1, 1); vertex 1 = bottom-left: (0, 1); vertex 3 = top-right: (1, 0)
        assertArrayEquals(new double[] {1, 1}, q.quad.uv(0), 1e-6);
        assertArrayEquals(new double[] {0, 1}, q.quad.uv(1), 1e-6);
        assertArrayEquals(new double[] {0, 0}, q.quad.uv(2), 1e-6);
        assertArrayEquals(new double[] {1, 0}, q.quad.uv(3), 1e-6);
    }

    @Test
    void flippedUGivesTheOtherUvOrderSoMirroringIsDetectable() {
        // the game swaps the U values for every other pair of fire layers
        FireQuadAssembler.Assembled q = feed(assembler(), 0.20f, 0.10f, 0.30f, 0.40f);
        assertArrayEquals(new double[] {0, 1}, q.quad.uv(0), 1e-6);
        assertArrayEquals(new double[] {1, 1}, q.quad.uv(1), 1e-6);
    }

    @Test
    void consecutiveQuadsAreAssembledIndependently() {
        FireQuadAssembler a = assembler();
        assertEquals(0, feed(a, 0.10f, 0.20f, 0.30f, 0.40f).sheet);
        assertEquals(1, feed(a, 0.50f, 0.60f, 0.30f, 0.40f).sheet);
        assertEquals(0, feed(a, 0.20f, 0.10f, 0.30f, 0.40f).sheet);
    }

    // ---------- failures ----------

    @Test
    void aQuadOnAnotherSpriteIsDropped() {
        assertNull(feed(assembler(), 0.70f, 0.80f, 0.30f, 0.40f));
        // and the assembler is ready for the next one
        assertNotNull(feed(assembler(), 0.10f, 0.20f, 0.30f, 0.40f));
    }

    @Test
    void verticesStraddlingTwoSpritesAreDropped() {
        FireQuadAssembler a = assembler();
        a.vertex(1, -1, -3, 0.55f, 0.40f);
        a.vertex(-1, -1, -3, 0.15f, 0.40f); // fire_1 then fire_0
        a.vertex(-1, 1, -3, 0.15f, 0.30f);
        assertNull(a.vertex(1, 1, -3, 0.55f, 0.30f));
    }

    @Test
    void resetDiscardsAPartialQuad() {
        FireQuadAssembler a = assembler();
        a.vertex(1, -1, -3, 0.20f, 0.40f);
        a.vertex(-1, -1, -3, 0.10f, 0.40f);
        a.reset();
        // a fresh quad must not be polluted by the two stale vertices
        FireQuadAssembler.Assembled q = feed(a, 0.10f, 0.20f, 0.30f, 0.40f);
        assertNotNull(q);
        assertArrayEquals(new double[] {1, -1, -3}, q.quad.vertex(0), 1e-9);
    }

    @Test
    void nonFiniteVertexMakesTheQuadDropped() {
        FireQuadAssembler a = assembler();
        a.vertex(Double.NaN, -1, -3, 0.20f, 0.40f);
        a.vertex(-1, -1, -3, 0.10f, 0.40f);
        a.vertex(-1, 1, -3, 0.10f, 0.30f);
        assertNull(a.vertex(1, 1, -3, 0.20f, 0.30f));
    }

    @Test
    void degenerateSpriteRectIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> new SpriteRect(0.2f, 0.2f, 0.3f, 0.4f));
        assertThrows(IllegalArgumentException.class, () -> new SpriteRect(0.1f, 0.2f, 0.4f, 0.3f));
        assertThrows(IllegalArgumentException.class, () -> new FireQuadAssembler(null, S1));
    }
}
