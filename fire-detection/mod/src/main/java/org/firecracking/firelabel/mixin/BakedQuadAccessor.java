package org.firecracking.firelabel.mixin;

import net.minecraft.client.render.model.BakedQuad;
import net.minecraft.client.texture.Sprite;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/** Which atlas sprite a baked quad uses (fire_0 vs fire_1). */
@Mixin(BakedQuad.class)
public interface BakedQuadAccessor {
    @Accessor("sprite")
    Sprite firelabel$getSprite();
}
