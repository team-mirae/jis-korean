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
			Map.entry("Left", new int[] {80, 0}));

	private Staging staging;
	/** The text field, as a game text box would end up after the inserts and passed keys. */
	private StringBuilder field;
	/** Keys the staging area let through to the game. */
	private StringBuilder passed;

	@BeforeEach
	void setUp() {
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
	void shiftOnANonTenseKeyIsEnglish() {
		check("Hi ", "Hi ");
		check("Steve ", "Steve ");
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
	void karabinerOptionYenIsRo() {
		check("{AltYen}{Ro} ", "\\\\ ");
	}

	@Test
	void dedicatedKeysAndEscapeWork() {
		check("1k ", "빠 ");
		check("{F8}1{S-F8}1k ", "1빠 ");
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
