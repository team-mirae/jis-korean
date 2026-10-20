package me.sjun.jiskorean.client;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenKeyboardEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.components.events.ContainerEventHandler;
import net.minecraft.client.gui.components.events.GuiEventListener;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.CharacterEvent;
import net.minecraft.client.input.KeyEvent;

import me.sjun.jiskorean.Staging;
import me.sjun.jiskorean.engine.Keys;
import me.sjun.jiskorean.engine.WasmEngine;
import me.sjun.jiskorean.rank.Ranker;

/**
 * JIS Korean in Minecraft text fields. It's always on: key presses in a focused {@link EditBox} go
 * to the staging area instead of the game. The mod runs its own engine on the raw key presses and
 * keeps the OS text input off for those fields ({@code TextInputManagerMixin}), so an OS input
 * method, JIS Korean included, never composes the same keys a second time; any character the OS
 * still sends to such a field is dropped. The layout mode follows the JIS Korean input method's
 * ({@link SystemMode}). See {@link Staging} for the behavior and {@link CandidateWindow} for the UI.
 */
public final class JisKoreanClient implements ClientModInitializer {
	public static final String MOD_ID = "jiskorean";

	private static Staging staging;
	private static EditBox field;
	/** A mode set with {@link #pinMode}, which wins over the input method's; -1 for none. */
	private static int pinnedMode = -1;

	@Override
	public void onInitializeClient() {
		staging = new Staging(WasmEngine.load(), Ranker.load());
		SystemMode.refresh();

		ScreenEvents.AFTER_INIT.register((client, screen, width, height) -> {
			// The input method's mode may have changed since the last screen.
			SystemMode.refresh();
			ScreenKeyboardEvents.allowKeyPress(screen).register(JisKoreanClient::onKey);
			ScreenKeyboardEvents.allowCharType(screen).register(JisKoreanClient::onCharacter);
			ScreenEvents.afterExtract(screen).register((s, graphics, mouseX, mouseY, delta) -> {
				EditBox focused = focusedField(s);
				if (focused != null) {
					CandidateWindow.draw(graphics, focused, staging);
				}
			});
			ScreenEvents.remove(screen).register(s -> {
				staging.clear();
				field = null;
			});
		});
	}

	/**
	 * Types in this mode (0 Mode A, JIS; 1 Mode B, US ANSI) instead of following the input method,
	 * or follows it again for -1. For tests.
	 */
	public static void pinMode(int mode) {
		pinnedMode = mode;
	}

	private static boolean onKey(Screen screen, KeyEvent event) {
		EditBox focused = focusedField(screen);
		if (focused == null) {
			return true;
		}
		if (focused != field) {
			// Focus moved to another field: the staging area belonged to the old one.
			staging.clear();
			field = focused;
		}
		staging.setMode(pinnedMode >= 0 ? pinnedMode : SystemMode.get());
		int modifiers = event.modifiers();
		// Also trust the held-key state for Shift: it decides between ㅂ and ㅃ.
		if (Minecraft.getInstance().hasShiftDown()) {
			modifiers |= Keys.MOD_SHIFT;
		}
		Staging.Result result = staging.key(event.key(), modifiers, before(focused));
		if (!result.insert().isEmpty()) {
			focused.insertText(result.insert());
		}
		return !result.consumed();
	}

	/** Text from the OS never reaches a field the mod types into: only the mod's engine composes. */
	private static boolean onCharacter(Screen screen, CharacterEvent event) {
		return focusedField(screen) == null;
	}

	/** The focused text field that takes input, looking through nested containers. */
	public static EditBox focusedField(Screen screen) {
		GuiEventListener focused = screen.getFocused();
		while (focused instanceof ContainerEventHandler container && !(focused instanceof EditBox)) {
			focused = container.getFocused();
		}
		return focused instanceof EditBox box && box.canConsumeInput() ? box : null;
	}

	private static String before(EditBox box) {
		String value = box.getValue();
		return value.substring(0, Math.min(box.getCursorPosition(), value.length()));
	}
}
