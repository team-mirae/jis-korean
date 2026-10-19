package me.sjun.jiskorean.client;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Properties;

import com.mojang.blaze3d.platform.InputConstants;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenKeyboardEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.components.events.ContainerEventHandler;
import net.minecraft.client.gui.components.events.GuiEventListener;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.CharacterEvent;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.resources.Identifier;

import me.sjun.jiskorean.Staging;
import me.sjun.jiskorean.engine.Keys;
import me.sjun.jiskorean.engine.WasmEngine;
import me.sjun.jiskorean.rank.Ranker;

/**
 * JIS Korean in Minecraft text fields. While it's on, key presses in a focused {@link EditBox}
 * go to the staging area instead of the game, and the game's own character events are dropped
 * for the keys it took. See {@link Staging} for the behavior and {@link CandidateWindow} for the UI.
 */
public final class JisKoreanClient implements ClientModInitializer {
	public static final String MOD_ID = "jiskorean";

	private static final KeyMapping.Category CATEGORY =
			KeyMapping.Category.register(Identifier.fromNamespaceAndPath(MOD_ID, "jiskorean"));
	/** On: JIS Korean. Default F18, the HHKB key in the hhkb-ekj-trilingual setup. */
	private static final KeyMapping KOREAN =
			new KeyMapping("key.jiskorean.korean", InputConstants.Type.KEYBOARD, Keys.F18, CATEGORY);
	/** Off: the game's own typing. Default 英数 (LANG2). */
	private static final KeyMapping ENGLISH =
			new KeyMapping("key.jiskorean.english", InputConstants.Type.KEYBOARD, Keys.LANG2, CATEGORY);

	private static boolean enabled;
	private static Staging staging;
	private static EditBox field;
	/** The last key press in a field was taken by the staging area, so its character must not land. */
	private static boolean swallowCharacters;

	@Override
	public void onInitializeClient() {
		staging = new Staging(WasmEngine.load(), Ranker.load());
		enabled = loadEnabled();
		KeyMappingHelper.registerKeyMapping(KOREAN);
		KeyMappingHelper.registerKeyMapping(ENGLISH);

		ClientTickEvents.END_CLIENT_TICK.register(client -> {
			while (KOREAN.consumeClick()) {
				setEnabled(true);
			}
			while (ENGLISH.consumeClick()) {
				setEnabled(false);
			}
		});

		ScreenEvents.AFTER_INIT.register((client, screen, width, height) -> {
			ScreenKeyboardEvents.allowKeyPress(screen).register(JisKoreanClient::onKey);
			ScreenKeyboardEvents.allowCharType(screen).register(JisKoreanClient::onCharacter);
			ScreenEvents.afterExtract(screen).register((s, graphics, mouseX, mouseY, delta) -> {
				EditBox focused = focusedField(s);
				if (enabled && focused != null) {
					CandidateWindow.draw(graphics, focused, staging);
				}
			});
			ScreenEvents.remove(screen).register(s -> {
				staging.clear();
				field = null;
			});
		});
	}

	public static boolean enabled() {
		return enabled;
	}

	private static boolean onKey(Screen screen, KeyEvent event) {
		EditBox focused = focusedField(screen);
		if (KOREAN.matches(event)) {
			setEnabled(true);
			return false;
		}
		if (ENGLISH.matches(event)) {
			if (focused != null && enabled) {
				focused.insertText(staging.commitRanked(before(focused)));
			}
			setEnabled(false);
			return false;
		}
		if (!enabled || focused == null) {
			swallowCharacters = false;
			return true;
		}
		if (focused != field) {
			// Focus moved to another field: the staging area belonged to the old one.
			staging.clear();
			field = focused;
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
		swallowCharacters = result.consumed();
		return !result.consumed();
	}

	private static boolean onCharacter(Screen screen, CharacterEvent event) {
		return !(enabled && focusedField(screen) != null && swallowCharacters);
	}

	/** Turns the mod on or off, as the two key mappings do. */
	public static void setEnabled(boolean on) {
		if (on && !enabled) {
			staging.activate();
		} else if (!on) {
			staging.clear();
		}
		if (on != enabled) {
			enabled = on;
			saveEnabled();
		}
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

	// --- The on/off state survives restarts.

	private static Path configFile() {
		return FabricLoader.getInstance().getConfigDir().resolve(MOD_ID + ".properties");
	}

	private static boolean loadEnabled() {
		Properties properties = new Properties();
		try (Reader reader = Files.newBufferedReader(configFile())) {
			properties.load(reader);
		} catch (IOException e) {
			return true;
		}
		return Boolean.parseBoolean(properties.getProperty("enabled", "true"));
	}

	private static void saveEnabled() {
		Properties properties = new Properties();
		properties.setProperty("enabled", Boolean.toString(enabled));
		try (Writer writer = Files.newBufferedWriter(configFile())) {
			properties.store(writer, "JIS Korean");
		} catch (IOException e) {
			// Not fatal: the state just isn't remembered.
		}
	}
}
