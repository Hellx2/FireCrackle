package org.firecracking.firelabel;

import com.mojang.blaze3d.systems.RenderSystem;
import org.firecracking.firelabel.core.StripPattern;
import org.lwjgl.opengl.GL11;

/**
 * Draws the sync strip in framebuffer pixels (independent of GUI scale) using GL scissor + clear, so every
 * cell is pixel-exact and fully opaque.
 */
final class StripRenderer {
    private StripRenderer() {}

    /** {@code tick} must already be wrapped with {@link StripPattern#wrapTick(long)}. */
    static void draw(int fbWidth, int fbHeight, int tick, int frameId) {
        int[] size = StripPattern.stripSize(fbWidth);
        if (size == null || fbHeight < size[1]) {
            return; // frame too small for the strip
        }
        boolean[] cells = StripPattern.cells(tick, frameId);

        boolean scissorWasOn = GL11.glIsEnabled(GL11.GL_SCISSOR_TEST);
        int[] oldBox = new int[4];
        GL11.glGetIntegerv(GL11.GL_SCISSOR_BOX, oldBox);

        RenderSystem.colorMask(true, true, true, true);
        GL11.glEnable(GL11.GL_SCISSOR_TEST);
        int y = fbHeight - size[1]; // GL origin is bottom-left; the strip sits at the top-left
        for (int i = 0; i < StripPattern.CELLS; i++) {
            int[] r = StripPattern.cellRange(fbWidth, i);
            float v = cells[i] ? 1.0f : 0.0f;
            GL11.glScissor(r[0], y, r[1] - r[0], size[1]);
            RenderSystem.clearColor(v, v, v, 1.0f);
            RenderSystem.clear(GL11.GL_COLOR_BUFFER_BIT, false);
        }

        GL11.glScissor(oldBox[0], oldBox[1], oldBox[2], oldBox[3]);
        if (!scissorWasOn) {
            GL11.glDisable(GL11.GL_SCISSOR_TEST);
        }
    }
}
