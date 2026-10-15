package org.firecracking.firelabel;

import net.minecraft.client.render.model.ModelLoader;
import net.minecraft.client.texture.Sprite;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityType;
import net.minecraft.util.math.Matrix4f;
import net.minecraft.util.math.Vector4f;
import org.firecracking.firelabel.core.FireQuadAssembler;
import org.firecracking.firelabel.core.SpriteRect;

/**
 * Receives the vertices of burning-entity fire as the game emits them (EntityRenderDispatcher.drawFireVertex),
 * puts them in view space and assembles quads. Only active during the world pass.
 */
public final class EntityFireCapture {
    private static Entity current;
    private static FireQuadAssembler assembler;
    private static Sprite assemblerSprite0;

    private EntityFireCapture() {}

    public static void begin(Entity entity) {
        current = entity;
        if (assembler != null) {
            assembler.reset();
        }
    }

    public static void end() {
        current = null;
        if (assembler != null) {
            assembler.reset();
        }
    }

    /** {@code model} maps the vertex's local position to view space; (x, y, z) is that local position. */
    public static void vertex(Matrix4f model, float x, float y, float z, float u, float v) {
        if (current == null || !FrameCapture.isWorldArmed()) {
            return;
        }
        FireQuadAssembler a = assembler();
        Vector4f p = new Vector4f(x, y, z, 1.0f);
        p.transform(model);
        FireQuadAssembler.Assembled quad = a.vertex(p.getX(), p.getY(), p.getZ(), u, v);
        if (quad != null) {
            FrameCapture.addEntityQuad(
                    current.getEntityId(), EntityType.getId(current.getType()).toString(), quad.sheet, quad.quad);
        }
    }

    private static FireQuadAssembler assembler() {
        Sprite s0 = ModelLoader.FIRE_0.getSprite();
        if (assembler == null || s0 != assemblerSprite0) {
            Sprite s1 = ModelLoader.FIRE_1.getSprite();
            assembler = new FireQuadAssembler(
                    new SpriteRect(s0.getMinU(), s0.getMaxU(), s0.getMinV(), s0.getMaxV()),
                    new SpriteRect(s1.getMinU(), s1.getMaxU(), s1.getMinV(), s1.getMaxV()));
            assemblerSprite0 = s0;
        }
        return assembler;
    }
}
