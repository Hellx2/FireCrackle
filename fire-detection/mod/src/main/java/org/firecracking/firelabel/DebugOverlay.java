package org.firecracking.firelabel;

import com.mojang.blaze3d.systems.RenderSystem;
import org.firecracking.firelabel.core.BBox;
import org.firecracking.firelabel.core.FirePart;
import org.firecracking.firelabel.core.FireSourceLabel;
import org.firecracking.firelabel.core.FrameRecord;
import org.lwjgl.opengl.GL11;

/**
 * Dev-only: draws the labels as coloured outlines so they can be checked against the flames by eye.
 * White (thick) = the ONE box of each fire block / entity / overlay. Thin coloured boxes inside it are its
 * parts: red = fire_0, green = fire_1, magenta/cyan = the mirrored variants.
 * Never enabled in normal runs, because it would be recorded into the video.
 */
final class DebugOverlay {
    private static final int PART_THICKNESS = 2;
    private static final int SOURCE_THICKNESS = 3;

    private DebugOverlay() {}

    static void draw(int fbWidth, int fbHeight, FrameRecord record) {
        if (!record.fire()) {
            return;
        }
        boolean scissorWasOn = GL11.glIsEnabled(GL11.GL_SCISSOR_TEST);
        int[] oldBox = new int[4];
        GL11.glGetIntegerv(GL11.GL_SCISSOR_BOX, oldBox);
        RenderSystem.colorMask(true, true, true, true);
        GL11.glEnable(GL11.GL_SCISSOR_TEST);

        for (FireSourceLabel s : record.fires()) {
            // the white source box goes first and a little OUTSIDE the label, so that a source with a single
            // variant (part box == source box) still shows its coloured box on top instead of hiding it
            BBox outer = new BBox();
            BBox label = s.box();
            outer.add(label.x0() - SOURCE_THICKNESS, label.y0() - SOURCE_THICKNESS);
            outer.add(label.x1() + SOURCE_THICKNESS, label.y1() + SOURCE_THICKNESS);
            outline(fbWidth, fbHeight, outer, new float[] {1, 1, 1}, SOURCE_THICKNESS);
            for (FirePart p : s.parts()) {
                float[] c = p.sheet == 0
                        ? (p.mirrored ? new float[] {1, 0, 1} : new float[] {1, 0, 0})
                        : (p.mirrored ? new float[] {0, 1, 1} : new float[] {0, 1, 0});
                outline(fbWidth, fbHeight, p.box(), c, PART_THICKNESS);
            }
        }

        GL11.glScissor(oldBox[0], oldBox[1], oldBox[2], oldBox[3]);
        if (!scissorWasOn) {
            GL11.glDisable(GL11.GL_SCISSOR_TEST);
        }
    }

    private static void outline(int fbWidth, int fbHeight, BBox box, float[] color, int t) {
        int x0 = (int) Math.floor(box.x0());
        int y0 = (int) Math.floor(box.y0());
        int x1 = (int) Math.ceil(box.x1());
        int y1 = (int) Math.ceil(box.y1());
        rect(fbWidth, fbHeight, x0, y0, x1 - x0, t, color); // top
        rect(fbWidth, fbHeight, x0, y1 - t, x1 - x0, t, color); // bottom
        rect(fbWidth, fbHeight, x0, y0, t, y1 - y0, color); // left
        rect(fbWidth, fbHeight, x1 - t, y0, t, y1 - y0, color); // right
    }

    /** Pixel rectangle with a TOP-left origin, clipped to the frame. */
    private static void rect(int fbWidth, int fbHeight, int x, int y, int w, int h, float[] color) {
        int cx0 = Math.max(x, 0);
        int cy0 = Math.max(y, 0);
        int cx1 = Math.min(x + w, fbWidth);
        int cy1 = Math.min(y + h, fbHeight);
        if (cx1 <= cx0 || cy1 <= cy0) {
            return;
        }
        GL11.glScissor(cx0, fbHeight - cy1, cx1 - cx0, cy1 - cy0); // GL origin is bottom-left
        RenderSystem.clearColor(color[0], color[1], color[2], 1.0f);
        RenderSystem.clear(GL11.GL_COLOR_BUFFER_BIT, false);
    }
}
