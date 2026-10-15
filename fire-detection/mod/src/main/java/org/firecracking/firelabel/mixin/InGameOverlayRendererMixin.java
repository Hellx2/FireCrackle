package org.firecracking.firelabel.mixin;

import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.hud.InGameOverlayRenderer;
import net.minecraft.client.util.math.MatrixStack;
import org.firecracking.firelabel.FrameCapture;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** First-person "you are on fire" overlay: two fire_1 quads drawn over everything. */
@Mixin(InGameOverlayRenderer.class)
public class InGameOverlayRendererMixin {
    @Inject(method = "renderFireOverlay", at = @At("HEAD"))
    private static void firelabel$fireOverlay(MinecraftClient client, MatrixStack matrices, CallbackInfo ci) {
        FrameCapture.captureOverlay(matrices.peek().getModel());
    }
}
