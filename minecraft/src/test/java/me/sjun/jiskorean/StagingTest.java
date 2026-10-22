package me.sjun.jiskorean;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import me.sjun.jiskorean.engine.Keys;
import me.sjun.jiskorean.engine.WasmEngine;
import me.sjun.jiskorean.rank.Ranker;

/**
 * The staging flow end to end: keys in, text out, through the real wasm engine and ranker.
 *
 * <p>Key strings: a–z type the key unshifted, A–Z with Shift, 0–9 the number row, a space is the
 * Space key, and {Name} is a named key (Enter, Tab, Escape, Backspace, F6–F10, Ctrl-A, AltYen).
 */
class StagingTest {
	private static final WasmEngine ENGINE = WasmEngine.load();
	private static final Ranker RANKER = Ranker.load();

	private static final Map<String, int[]> NAMED = Map.ofEntries(
			Map.entry("Enter", new int[] {Keys.RETURN, 0}),
			Map.entry("Tab", new int[] {Keys.TAB, 0}),
			Map.entry("Escape", new int[] {Keys.ESCAPE, 0}),
			Map.entry("Backspace", new int[] {Keys.BACKSPACE, 0}),
			Map.entry("F6", new int[] {Keys.F6, 0}),
			Map.entry("F7", new int[] {Keys.F7, 0}),
			Map.entry("F8", new int[] {65, 0}),
			Map.entry("S-F8", new int[] {65, Keys.MOD_SHIFT}),
			Map.entry("F10", new int[] {67, 0}),
			Map.entry("Minus", new int[] {45, 0}),
			Map.entry("Slash", new int[] {56, 0}),
			Map.entry("Ro", new int[] {Keys.INTERNATIONAL1, 0}),
			Map.entry("AltYen", new int[] {Keys.INTERNATIONAL3, Keys.MOD_ALT}),
			Map.entry("Ctrl-A", new int[] {4, Keys.MOD_CONTROL}),
			Map.entry("Left", new int[] {80, 0}),
			Map.entry("S-2", new int[] {31, Keys.MOD_SHIFT}),
			Map.entry("S-4", new int[] {33, Keys.MOD_SHIFT}),
			Map.entry("S-6", new int[] {35, Keys.MOD_SHIFT}),
			Map.entry("S-Minus", new int[] {45, Keys.MOD_SHIFT}),
			Map.entry("Grave", new int[] {53, 0}));

	private Staging staging;
	/** The text field, as a game text box would end up after the inserts and passed keys. */
	private StringBuilder field;
	/** Keys the staging area let through to the game. */
	private StringBuilder passed;

	@BeforeEach
	void setUp() {
		ENGINE.setMode(0);
		ENGINE.clear();
		ENGINE.activate();
		staging = new Staging(ENGINE, RANKER);
		field = new StringBuilder();
		passed = new StringBuilder();
	}

	private void type(String keys) {
		for (int i = 0; i < keys.length(); i++) {
			char c = keys.charAt(i);
			int scancode;
			int mods = 0;
			String name = null;
			if (c == '{') {
				int end = keys.indexOf('}', i);
				name = keys.substring(i + 1, end);
				int[] key = NAMED.get(name);
				scancode = key[0];
				mods = key[1];
				i = end;
			} else if (c == ' ') {
				scancode = Keys.SPACE;
			} else if (c >= 'a' && c <= 'z') {
				scancode = 4 + c - 'a';
			} else if (c >= 'A' && c <= 'Z') {
				scancode = 4 + c - 'A';
				mods = Keys.MOD_SHIFT;
			} else if (c >= '1' && c <= '9') {
				scancode = 30 + c - '1';
			} else if (c == '0') {
				scancode = 39;
			} else {
				throw new IllegalArgumentException("key " + c);
			}
			Staging.Result result = staging.key(scancode, mods, field.toString());
			field.append(result.insert());
			if (!result.consumed()) {
				if (scancode == Keys.BACKSPACE && !field.isEmpty()) {
					field.setLength(field.length() - 1);
				}
				passed.append(name != null ? "{" + name + "}" : String.valueOf(c));
			}
		}
	}

	private void check(String keys, String expected) {
		setUp();
		type(keys);
		assertEquals(expected, field.toString(), "keys: " + keys);
	}

	@Test
	void koreanWordsCommitAsKorean() {
		check("gksrmf ", "한글 ");
		check("kssudgktpy ", "안녕하세요 ");
		check("akfll ", "말이 ");
		check("gkrryptj ", "학교에서 ");
		check("rkatkgkqslek ", "감사합니다 ");
	}

	@Test
	void englishWordsCommitAsEnglish() {
		check("hello ", "hello ");
		check("minecraft ", "minecraft ");
		check("diamond ", "diamond ");
		check("lol ", "lol ");
	}

	@Test
	void sentencesSwitchWordByWord() {
		check("dhsmf eklk ", "오늘 다이아 ");
		check("let me craft a sword ", "let me craft a sword ");
		check("gksrmf diamond eklk ", "한글 diamond 다이아 ");
	}

	@Test
	void symbolsAndShiftedDigitsAloneCommitAsTyped() {
		// No letter keys: the legends' reading, not the stock JIS characters (&$ for 64, = for ?).
		check("{S-6}{S-4} ", "64 ");
		check("{S-Minus}{Enter}", "?");
		setUp();
		field.append("/tp ");
		type("{S-6}{S-4} ");
		assertEquals("/tp 64 ", field.toString());
	}

	@Test
	void shiftZStaysKieukInTheEscape() {
		// Spec §6: Shift+Z is F10 except in the F8 escape, where it is ㅋ.
		check("{F8}zZ{F6}", "ㅋㅋ");
	}

	@Test
	void shiftOnANonTenseKeyIsEnglish() {
		check("Hi ", "Hi ");
		check("Steve ", "Steve ");
	}

	@Test
	void sloppyShiftOnAKnownKoreanWordStaysKorean() {
		// Shift still held after ㅆ (⇧T) over the next key: 있는, not dlTSms.
		check("dlTSms ", "있는 ");
	}

	@Test
	void shiftVowelMidWordIsKorean() {
		// Spec §4.2: ⇧ㅣ starts a syllable on ㅇ, so 말이 is a k f ⇧L and 오이 is h ⇧L.
		check("akfL ", "말이 ");
		check("tkfkaL ", "사람이 ");
		check("hL ", "오이 ");
	}

	@Test
	void f7ForcesEnglishAndIsRemembered() {
		check("dkssud{F7}", "dkssud");
		type(" dkssud ");
		assertEquals("dkssud dkssud ", field.toString());
	}

	@Test
	void f6ForcesKoreanAndIsRemembered() {
		check("world{F6}", "재깅");
		type(" world ");
		assertEquals("재깅 재깅 ", field.toString());
	}

	@Test
	void f6IsTheEnginesF6() {
		// Inherited from the engine (spec §6): F6 drops a plain implied ㅇ on the last syllable.
		check("ok{F6}", "애ㅏ");
	}

	@Test
	void enterCommitsAndPassesTabCommitsOnly() {
		check("gksrmf{Enter}", "한글");
		assertEquals("{Enter}", passed.toString());
		check("gksrmf{Tab}", "한글");
		assertEquals("", passed.toString());
		check("{Tab}", "");
		assertEquals("{Tab}", passed.toString());
	}

	@Test
	void escapeDropsTheStagingAreaFirst() {
		check("gksrmf{Escape}", "");
		assertEquals("", passed.toString());
		type("{Escape}");
		assertEquals("{Escape}", passed.toString());
	}

	@Test
	void backspaceEditsStagedJamoThenPasses() {
		check("gksrmf{Backspace} ", "한그 ");
		check("{Backspace}", "");
		assertEquals("{Backspace}", passed.toString());
	}

	@Test
	void shortcutsAndArrowsCommitFirst() {
		check("gksrmf{Ctrl-A}", "한글");
		assertEquals("{Ctrl-A}", passed.toString());
		check("hello{Left}", "hello");
	}

	@Test
	void commandNamesAreEnglishArgumentsAreRanked() {
		setUp();
		field.append("/");
		type("rkwk ");
		assertEquals("/rkwk ", field.toString(), "command name stays English");
		type("dkssud ");
		assertEquals("/rkwk 안녕 ", field.toString(), "arguments are ranked");
	}

	@Test
	void koreanCommandAliasesAreKorean() {
		// The Mirae plugin's Korean aliases (tools/mirae/commands.txt): /귓말 is r n l t a k f.
		setUp();
		field.append("/");
		type("rnltakf ");
		assertEquals("/귓말 ", field.toString(), "a Korean alias is Korean");
		setUp();
		field.append("/");
		type("tmvhs ");
		assertEquals("/스폰 ", field.toString());
	}

	@Test
	void karabinerOptionYenIsRo() {
		check("{AltYen}{Ro} ", "\\\\ ");
	}

	@Test
	void shiftZIsF10() {
		// Shift+Z converts to half-width letters, as F10 does (spec §6), so English needs no F-key.
		check("gksZ ", "gks ");
		check("gksZZ ", "GKS ");
	}

	@Test
	void shiftZSkipsAheadWhenEnglishIsAlreadyRanked() {
		// hello already ranks English, exactly what F10 gives: Shift+Z goes on to HELLO.
		check("helloZ ", "HELLO ");
		check("helloZZ ", "Hello ");
	}

	@Test
	void dedicatedKeysAndEscapeWork() {
		check("1k ", "빠 ");
		check("{F8}1{S-F8}1k ", "1빠 ");
	}

	@Test
	void modeBTypesUsSymbols() {
		// Mode A reads the keys as JIS (Shift+2 is "), Mode B as US ANSI (Shift+2 is @, and ` has a key).
		check("{F8}{S-2} ", "\" ");
		setUp();
		staging.setMode(1);
		type("{F8}{S-2}{Grave} ");
		assertEquals("@` ", field.toString());
		// Korean is the same in both modes.
		setUp();
		staging.setMode(1);
		type("gksrmf ");
		assertEquals("한글 ", field.toString());
	}

	@Test
	void stagingShowsBothReadings() {
		setUp();
		type("gksrmf");
		assertEquals("한글", staging.korean());
		assertEquals("gksrmf", staging.english());
		assertEquals(Ranker.Language.KOREAN, staging.ranking("").language());
		assertTrue(staging.ranking("").margin() > 0);
	}
}
