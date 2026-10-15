package org.firecracking.firelabel.mixin;

import net.minecraft.client.render.VertexConsumer;
import net.minecraft.client.render.VertexConsumerProvider;
import net.minecraft.client.render.entity.EntityRenderDispatcher;
import net.minecraft.client.util.math.MatrixStack;
import net.minecraft.entity.Entity;
import org.firecracking.firelabel.EntityFireCapture;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Burning entities: the game draws stacked fire quads; we copy the exact vertices it emits. */
@Mixin(EntityRenderDispatcher.class)
public class EntityRenderDispatcherMixin {
    @Inject(method = "renderFire", at = @At("HEAD"))
    private void firelabel$beginFire(
            MatrixStack matrices, VertexConsumerProvider vertexConsumers, Entity entity, CallbackInfo ci) {
        EntityFireCapture.begin(entity);
    }

    @Inject(method = "renderFire", at = @At("RETURN"))
    private void firelabel$endFire(
            MatrixStack matrices, VertexConsumerProvider vertexConsumers, Entity entity, CallbackInfo ci) {
        EntityFireCapture.end();
    }

    @Inject(method = "drawFireVertex", at = @At("HEAD"))
    private static void firelabel$fireVertex(
            MatrixStack.Entry entry,
            VertexConsumer vertices,
            float x,
            float y,
            float z,
            float u,
            float v,
            CallbackInfo ci) {
        EntityFireCapture.vertex(entry.getModel(), x, y, z, u, v);
    }
}
