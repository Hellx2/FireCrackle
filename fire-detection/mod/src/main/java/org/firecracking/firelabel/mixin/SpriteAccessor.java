package org.firecracking.firelabel.mixin;

import net.minecraft.client.resource.metadata.AnimationResourceMetadata;
import net.minecraft.client.texture.Sprite;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/** Reads the private animation state of an atlas sprite. */
@Mixin(Sprite.class)
public interface SpriteAccessor {
    /** The animation step (index into the animation's frame order), NOT the image row. */
    @Accessor("frameIndex")
    int firelabel$getFrameIndex();

    /** Holds the frame order from the .mcmeta; maps a step to the displayed image row. */
    @Accessor("animationMetadata")
    AnimationResourceMetadata firelabel$getAnimationMetadata();
}
