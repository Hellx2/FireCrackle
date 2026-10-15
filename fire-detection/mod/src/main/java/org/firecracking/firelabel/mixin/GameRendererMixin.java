package org.firecracking.firelabel.mixin;

import net.minecraft.client.render.GameRenderer;
import net.minecraft.client.util.math.MatrixStack;
import net.minecraft.util.math.Matrix4f;
import org.firecracking.firelabel.FireLabel;
import org.firecracking.firelabel.FrameCapture;
import org.firecracking.firelabel.GuiMask;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(GameRenderer.class)
public class GameRendererMixin {
    /** At the very end of the frame, so the strip is drawn over the world, HUD and GUI. */
    @Inject(method = "render", at = @At("TAIL"))
    private void firelabel$afterRender(float tickDelta, long startTime, boolean tick, CallbackInfo ci) {
        FireLabel.onFrameRendered();
    }

    /**
     * The world (hand and on-fire overlay included) is finished and the GUI has not started: clear framebuffer alpha
     * so that, once the GUI has drawn, alpha marks exactly the pixels it covers.
     */
    @Inject(method = "renderWorld", at = @At("RETURN"))
    private void firelabel$afterWorld(float tickDelta, long limitTime, MatrixStack matrices, CallbackInfo ci) {
        GuiMask.armAfterWorld();
    }

    /** 1.16.5 has no projection getter, and the hand pass uses a different projection than the world pass. */
    @Inject(method = "loadProjectionMatrix", at = @At("HEAD"))
    private void firelabel$noteProjection(Matrix4f matrix, CallbackInfo ci) {
        FrameCapture.noteProjection(matrix);
    }
}
