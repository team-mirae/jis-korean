package me.sjun.jiskorean;

import java.util.HashMap;
import java.util.Map;

import me.sjun.jiskorean.engine.Keys;
import me.sjun.jiskorean.engine.WasmEngine;
import me.sjun.jiskorean.rank.Ranker;
import me.sjun.jiskorean.rank.Ranker.Language;

/**
 * The staging area of one text field: the keys typed since the last commit, read two ways.
 * The engine keeps the Korean reading and the English reading of the same keys; Space, Enter and
 * Tab commit whichever the ranker picks. F6 forces Korean, F7 forces English (in place of the
 * engine's hanja), and both are remembered for the rest of the session.
 *
 * <p>No Minecraft types here, so the whole flow is unit-tested; the client code only moves key
 * events in and inserted text out.
 */
public final class Staging {
	/**
	 * What to do with a key press: insert {@code insert} into the field (possibly empty), then let
	 * the key through to the game unless {@code consumed}.
	 */
	public record Result(boolean consumed, String insert) {
		static final Result PASS = new Result(false, "");
		static final Result CONSUMED = new Result(true, "");
	}

	private final WasmEngine engine;
	/** The engine's layout mode: the engine id (spec §3.6.1), 0 for Mode A (JIS), the engine's default. */
	private int mode;
	private final Ranker ranker;
	private final Map<String, Language> overrides = new HashMap<>();
	private Language previous;
	private String korean = "";
	private String english = "";

	public Staging(WasmEngine engine, Ranker ranker) {
		this.engine = engine;
		this.ranker = ranker;
	}

	public boolean isEmpty() {
		return korean.isEmpty() && english.isEmpty();
	}

	public String korean() {
		return korean;
	}

	public String english() {
		return english;
	}

	public boolean escaped() {
		return engine.escaped();
	}

	/** The current ranking, for the candidate window. {@code before}: the field's text before the cursor. */
	public Ranker.Choice ranking(String before) {
		// A command name (/give, /tp) is always English; its arguments are ranked as usual.
		boolean commandName = (before + english).startsWith("/") && !before.contains(" ");
		return ranker.rank(korean, english, new Ranker.Context(commandName, previous, overrides));
	}

	/**
	 * One key press in a focused text field.
	 *
	 * @param scancode SDL scancode ({@code KeyEvent.key()})
	 * @param modifiers SDL modifier mask ({@code KeyEvent.modifiers()})
	 * @param before the field's text before the cursor (for command names)
	 */
	public Result key(int scancode, int modifiers, String before) {
		boolean shift = (modifiers & Keys.MOD_SHIFT) != 0;
		boolean alt = (modifiers & Keys.MOD_ALT) != 0;
		boolean shortcut = (modifiers & (Keys.MOD_CONTROL | Keys.MOD_SUPER)) != 0;
		// A Karabiner rule common on JIS Macs sends ろ as Option+¥; read it back as ろ.
		if (scancode == Keys.INTERNATIONAL3 && alt && !shortcut) {
			scancode = Keys.INTERNATIONAL1;
			alt = false;
		}
		String code = Keys.code(scancode);
		if (code.equals("Modifier")) {
			return Result.PASS;
		}
		if (shortcut || alt) {
			return new Result(false, commitRanked(before));
		}
		if (scancode == Keys.TAB) {
			// Commit without letting Tab through, so the game's own suggestions don't open.
			return isEmpty() ? Result.PASS : new Result(true, commitRanked(before));
		}
		switch (code) {
			case "Space":
				// Always typed here: the client blocks the game's own characters while the mod is on.
				return new Result(true, commitRanked(before) + " ");
			case "Enter":
				// Commit, then let Enter through (chat sends the line).
				return new Result(false, commitRanked(before));
			case "Escape":
				// The first Escape drops the staging area; the next one closes the screen.
				if (isEmpty()) {
					return Result.PASS;
				}
				clear();
				return Result.CONSUMED;
			case "F6":
				return isEmpty() ? Result.CONSUMED : new Result(true, commitForced(Language.KOREAN));
			case "F7":
				return isEmpty() ? Result.CONSUMED : new Result(true, commitForced(Language.ENGLISH));
			case "Backspace":
				if (isEmpty()) {
					return Result.PASS;
				}
				break;
			case "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Other":
				return new Result(false, commitRanked(before));
			default:
				break;
		}
		// Shift+Z is F10 (spec §6). When the ranked reading is already what F10 would give (English,
		// plain letters as typed), it skips ahead: F10 twice (gks → GKS).
		// Only on the first press: once converted, the two readings are the same and F10 just cycles.
		// Not in the F8 escape, where Shift+Z is ㅋ as in standard 2-set (spec §6).
		if (code.equals("KeyZ") && shift && !isEmpty() && !engine.escaped() && !korean.equals(english)
				&& english.chars().allMatch(Character::isLetter) && ranking(before).language() == Language.ENGLISH) {
			update(engine.key("F10", false));
		}
		WasmEngine.Output out = engine.key(code, shift);
		update(out);
		return new Result(out.consumed(), out.commit());
	}

	/** Commits the ranked reading. Call on focus loss and when the screen closes. */
	public String commitRanked(String before) {
		if (isEmpty()) {
			return "";
		}
		Language language = ranking(before).language();
		String text = language == Language.KOREAN ? korean : english;
		previous = language;
		clear();
		return text;
	}

	/** Drops the staging area without committing. */
	/**
	 * Switches the engine's layout mode by its engine id (spec §3.6.1), dropping anything
	 * staged. Does nothing if the mode is already set.
	 */
	public void setMode(int id) {
		if (id != mode) {
			clear();
			engine.setMode(id);
			mode = id;
		}
	}

	public int mode() {
		return mode;
	}

	public void clear() {
		engine.clear();
		korean = "";
		english = "";
	}

	/** The mod was switched on again: leave any F8 escape, start clean. */
	public void activate() {
		clear();
		engine.activate();
	}

	private String commitForced(Language language) {
		overrides.put(english.toLowerCase(), language);
		previous = language;
		String text;
		if (language == Language.KOREAN) {
			// The engine's F6: commits, dropping a plain implied ㅇ (spec §6).
			text = engine.key("F6", false).commit();
		} else {
			text = english;
		}
		clear();
		return text;
	}

	private void update(WasmEngine.Output out) {
		korean = out.preedit();
		english = out.english();
	}
}
