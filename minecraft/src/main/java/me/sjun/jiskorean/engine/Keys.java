package me.sjun.jiskorean.engine;

/**
 * SDL scancodes (Minecraft 26.3's {@code KeyEvent.key()}, USB HID positions) to the engine's key
 * names ({@code KeyboardEvent.code}, spec §2). Physical positions, so the OS layout doesn't matter.
 */
public final class Keys {
	public static final int SPACE = 44;
	public static final int RETURN = 40;
	public static final int KEYPAD_ENTER = 88;
	public static final int ESCAPE = 41;
	public static final int BACKSPACE = 42;
	public static final int TAB = 43;
	public static final int F6 = 63;
	public static final int F7 = 64;
	/** JIS ろ (international 1). */
	public static final int INTERNATIONAL1 = 135;
	/** JIS ¥ (international 3). */
	public static final int INTERNATIONAL3 = 137;
	/** JIS かな, Korean 한/영. */
	public static final int LANG1 = 144;
	/** JIS 英数, Korean 한자. */
	public static final int LANG2 = 145;
	public static final int F18 = 109;

	/** SDL modifier masks ({@code InputConstants.MOD_*}). */
	public static final int MOD_SHIFT = 3;
	public static final int MOD_CONTROL = 192;
	public static final int MOD_ALT = 768;
	public static final int MOD_SUPER = 3072;

	private Keys() {
	}

	/** The engine key name for a scancode: a key name, "Modifier", or "Other". */
	public static String code(int scancode) {
		if (scancode >= 4 && scancode <= 29) {
			return "Key" + (char) ('A' + scancode - 4);
		}
		if (scancode >= 30 && scancode <= 38) {
			return "Digit" + (scancode - 29);
		}
		return switch (scancode) {
			case 39 -> "Digit0";
			case 45 -> "Minus";
			case 46 -> "Equal";
			case INTERNATIONAL3 -> "IntlYen";
			case 47 -> "BracketLeft";
			case 48 -> "BracketRight";
			case 51 -> "Semicolon";
			case 53 -> "Backquote";
			case 100 -> "IntlBackslash";
			case 52 -> "Quote";
			// JIS ] arrives as backslash on macOS and as non-US # elsewhere.
			case 49, 50 -> "Backslash";
			case 54 -> "Comma";
			case 55 -> "Period";
			case 56 -> "Slash";
			case INTERNATIONAL1 -> "IntlRo";
			case F6 -> "F6";
			case F7 -> "F7";
			case 65 -> "F8";
			case 66 -> "F9";
			case 67 -> "F10";
			case SPACE -> "Space";
			case RETURN, KEYPAD_ENTER -> "Enter";
			case BACKSPACE -> "Backspace";
			case ESCAPE -> "Escape";
			case 79 -> "ArrowRight";
			case 80 -> "ArrowLeft";
			case 81 -> "ArrowDown";
			case 82 -> "ArrowUp";
			case 57, 224, 225, 226, 227, 228, 229, 230, 231 -> "Modifier";
			default -> "Other";
		};
	}
}
