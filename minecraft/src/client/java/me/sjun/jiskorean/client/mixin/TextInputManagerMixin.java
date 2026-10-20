package me.sjun.jiskorean.client.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import com.mojang.blaze3d.platform.TextInputManager;

import net.minecraft.client.gui.components.EditBox;

/**
 * Keeps the OS text input (SDL text input, and with it the OS input method) off for single-line
 * text fields, which the mod types into itself from raw key presses. With it on, an OS input
 * method such as JIS Korean would compose the same keys a second time. Other text widgets (signs,
 * books) keep the game's own text input.
 */
@Mixin(TextInputManager.class)
public abstract class TextInputManagerMixin {
	@Shadow
	public abstract void stopTextInput();

	@Inject(method = "startTextInput", at = @At("HEAD"), cancellable = true)
	private void jiskorean$keepOsInputOff(Object owner, CallbackInfo ci) {
		if (owner instanceof EditBox) {
			stopTextInput();
			ci.cancel();
		}
	}
}
