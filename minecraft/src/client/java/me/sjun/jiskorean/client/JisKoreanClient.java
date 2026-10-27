package me.sjun.jiskorean.client;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenKeyboardEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenMouseEvents;
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
	/** The field the staging area belongs to, and its screen. */
	private static EditBox field;
	private static Screen fieldScreen;
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
			// Before the click reaches a widget, so a button (Done, Create World), the anvil's output
			// slot or another field sees the staged word already in the field (spec §5.1: focus loss
			// commits). A click inside the same field commits too, then moves the caret, as an arrow
			// key does.
			ScreenMouseEvents.beforeMouseClick(screen).register((s, event) -> flush());
			ScreenEvents.afterExtract(screen).register((s, graphics, mouseX, mouseY, delta) -> {
				EditBox focused = focusedField(s);
				follow(s, focused);
				if (focused != null && focused == field) {
					CandidateWindow.draw(graphics, focused, staging);
				}
			});
			ScreenEvents.remove(screen).register(s -> {
				// Closed by the game or a click (Escape drops the staging area first): commit into the
				// field, whose value the screen may still read as it goes.
				if (s == fieldScreen) {
					flush();
					field = null;
					fieldScreen = null;
				}
			});
		});
	}

	/**
	 * Types in this mode, by its engine id (0 Mode A, 1 Mode B, 2 Mode C), instead of following the
	 * input method, or follows it again for -1. For tests.
	 */
	public static void pinMode(int mode) {
		pinnedMode = mode;
	}

	private static boolean onKey(Screen screen, KeyEvent event) {
		EditBox focused = focusedField(screen);
		if (focused == null) {
			return true;
		}
		follow(screen, focused);
		// Keys typed in the old mode are committed as typed before the switch.
		String switched = staging.setMode(pinnedMode >= 0 ? pinnedMode : Staging.koreanMode(SystemMode.get()), before(focused));
		if (!switched.isEmpty()) {
			focused.insertText(switched);
		}
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

	/**
	 * Keeps the staging area with the focused field. If focus moved to another field (Tab, or code
	 * moving it), what's staged is committed into the field it was typed in. If the screen rebuilt
	 * its widgets (a window resize), the old field is gone and the staging area moves on to the new
	 * one, which the screen filled with the old one's text.
	 */
	private static void follow(Screen screen, EditBox focused) {
		if (focused == null || focused == field) {
			return;
		}
		if (!(field != null && screen == fieldScreen && !contains(screen, field))) {
			flush();
		}
		field = focused;
		fieldScreen = screen;
	}

	/** Commits the staging area (ranked) into its field. */
	private static void flush() {
		if (field == null || staging.isEmpty()) {
			return;
		}
		String text = staging.commitRanked(before(field));
		if (!text.isEmpty()) {
			// Through the field's own insert, so its responder runs (the anvil sends the new name).
			field.insertText(text);
		}
	}

	/** Whether {@code target} is still one of the container's widgets, looking through nested containers. */
	private static boolean contains(ContainerEventHandler container, GuiEventListener target) {
		for (GuiEventListener child : container.children()) {
			if (child == target || child instanceof ContainerEventHandler nested && contains(nested, target)) {
				return true;
			}
		}
		return false;
	}

	private static String before(EditBox box) {
		String value = box.getValue();
		return value.substring(0, Math.min(box.getCursorPosition(), value.length()));
	}
}
