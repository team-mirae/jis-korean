package me.sjun.jiskorean.rank;

import java.io.BufferedReader;
import java.io.DataInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

/**
 * Decides whether a staging area is Korean or English.
 *
 * <p>Both candidates come from the same keystrokes, so the core is a likelihood comparison on those
 * keys: one key-trigram model trained on English spelling, one on the keys JIS Korean users press
 * for Korean words ({@code tools/build_ranker.py}, which also reports accuracy). Rules on top cover
 * what a model can't see: explicit overrides, Shift on keys Korean never shifts, exact word lists,
 * leftover bare jamo, command lines, and the language of the previous word.
 *
 * <p>The word lists add Minecraft-only vocabulary from 미래서버 (tools/mirae/README.md) when the jar
 * has it: words from the server's public chat, and the Mirae plugin's commands. A command name is
 * English, unless it's one of the plugin's Korean aliases (/귓말, /지역, /스폰).
 */
public final class Ranker {
	public enum Language { KOREAN, ENGLISH }

	/** The choice and its margin (positive = Korean, in bits per key). */
	public record Choice(Language language, double margin, String reason) {
	}

	/**
	 * What the ranker knows about the surroundings: whether the word is a command name (always
	 * English), the language of the previous word, and the user's F6/F7 choices this session.
	 */
	public record Context(boolean command, Language previous, Map<String, Language> overrides) {
		public static final Context NONE = new Context(false, null, Map.of());
	}

	private static final int SIZE = 27;
	private static final int START = 26;
	private static final int END = 26;
	/** Shifted letters that type a tense consonant or ㅒ ㅖ: Shift there can be Korean. */
	private static final String KOREAN_SHIFTS = "QWERTOP";
	/**
	 * Vowel keys whose Shift starts a syllable on ㅇ (spec §4.2): Korean mid-word (말이 =
	 * a k f ⇧L), so on those only a capital at the start of a word proves English.
	 */
	private static final String IEUNG_SHIFTS = "YUIHJKLBNM";
	/** Canonical 2-set keys for the dedicated number-row jamo (1–5 ㅃ ㅉ ㄸ ㄲ ㅆ, 6–0 ㅟ ㅝ ㅚ ㅢ ㅘ). */
	private static final String[] DIGIT_KEYS = {"hk", "q", "w", "e", "r", "t", "nl", "nj", "hl", "ml"};

	static final double LEXICON = 2.0;
	static final double STICKY = 0.5;
	static final double BARE_JAMO = 1.5;

	private final byte[] english;
	private final byte[] korean;
	private final Set<String> englishWords;
	private final Set<String> koreanWords;
	/** Command names that are Korean (the Mirae plugin's Korean aliases). */
	private final Set<String> koreanCommands;

	Ranker(byte[] english, byte[] korean, Set<String> englishWords, Set<String> koreanWords, Set<String> koreanCommands) {
		this.english = english;
		this.korean = korean;
		this.englishWords = englishWords;
		this.koreanWords = koreanWords;
		this.koreanCommands = koreanCommands;
	}

	/** Loads the model and word lists bundled with the mod. */
	public static Ranker load() {
		try (InputStream in = resource("ranker.bin")) {
			DataInputStream data = new DataInputStream(in);
			byte[] magic = data.readNBytes(4);
			if (!"JKRK".equals(new String(magic, StandardCharsets.US_ASCII)) || data.readUnsignedShort() != 1
					|| data.readUnsignedShort() != SIZE) {
				throw new IllegalStateException("ranker.bin: unknown format");
			}
			byte[] en = data.readNBytes(SIZE * SIZE * SIZE);
			byte[] ko = data.readNBytes(SIZE * SIZE * SIZE);
			resource("common-en.txt").close(); // required: fails loudly if the jar lacks them
			resource("common-ko.txt").close();
			Set<String> englishWords = words("common-en.txt");
			Set<String> koreanWords = words("common-ko.txt");
			Set<String> koreanCommands = new HashSet<>();
			// Minecraft-only vocabulary, when the jar has it (tools/mirae).
			englishWords.addAll(words("mirae-vocab-en.txt"));
			koreanWords.addAll(words("mirae-vocab-ko.txt"));
			for (String line : lines("mirae-commands.txt")) {
				String[] fields = line.split("\t");
				if (fields.length >= 2) {
					(fields[1].equals("ko") ? koreanCommands : englishWords).add(fields[0].toLowerCase());
				}
			}
			return new Ranker(en, ko, englishWords, koreanWords, koreanCommands);
		} catch (IOException e) {
			throw new UncheckedIOException(e);
		}
	}

	public Choice rank(String koreanText, String englishText, Context context) {
		if (koreanText.isEmpty() || koreanText.equals(englishText)) {
			return new Choice(Language.ENGLISH, 0, "same");
		}
		Language override = context.overrides().get(englishText.toLowerCase());
		if (override != null) {
			return new Choice(override, override == Language.KOREAN ? 99 : -99, "override");
		}
		// No letter or digit key at all (Shift+digits, ? ! ^^): the model has nothing to go on, and
		// what the keys' legends show is the Korean reading (64, not the stock JIS &$).
		if (englishText.chars().noneMatch(c -> c < 128 && Character.isLetterOrDigit(c))) {
			return new Choice(Language.KOREAN, 99, "symbols");
		}
		// A known Korean word is Korean even with Shift held over from a tense consonant (있는 typed
		// d l ⇧T ⇧S m s): sloppy Shift still types 2-set (spec §1).
		for (int i = 0; i < englishText.length() && !koreanWords.contains(koreanText); i++) {
			char c = englishText.charAt(i);
			boolean midWord = i > 0 && Character.isLetter(englishText.charAt(i - 1));
			if (c >= 'A' && c <= 'Z' && KOREAN_SHIFTS.indexOf(c) < 0
					&& !(midWord && IEUNG_SHIFTS.indexOf(c) >= 0)) {
				return new Choice(Language.ENGLISH, -99, "shift");
			}
		}

		double margin = keyMargin(englishText);
		String reason = "model";
		if (koreanWords.contains(koreanText)) {
			margin += LEXICON;
			reason = "ko word";
		}
		if (englishWords.contains(englishText.toLowerCase())) {
			margin -= LEXICON;
			reason = reason.equals("ko word") ? "both words" : "en word";
		}
		if (!koreanWords.contains(koreanText)) {
			long bare = koreanText.codePoints().filter(Ranker::isJamo).count();
			margin -= BARE_JAMO * bare;
		}
		if (context.command()) {
			return koreanCommands.contains(koreanText)
					? new Choice(Language.KOREAN, 99, "ko command")
					: new Choice(Language.ENGLISH, -99, "command");
		}
		if (context.previous() == Language.KOREAN) {
			margin += STICKY;
		} else if (context.previous() == Language.ENGLISH) {
			margin -= STICKY;
		}
		return new Choice(margin > 0 ? Language.KOREAN : Language.ENGLISH, margin, reason);
	}

	/** Korean minus English, in average log2 probability per key, over the letter runs. */
	double keyMargin(String englishText) {
		double total = 0;
		int keys = 0;
		StringBuilder run = new StringBuilder();
		for (char c : (englishText + " ").toCharArray()) {
			if (c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z') {
				run.append(Character.toLowerCase(c));
			} else if (c >= '0' && c <= '9') {
				run.append(DIGIT_KEYS[c - '0']);
			} else if (!run.isEmpty()) {
				total += logProbability(korean, run) - logProbability(english, run);
				keys += run.length() + 1;
				run.setLength(0);
			}
		}
		return keys == 0 ? 0 : total / keys;
	}

	private static double logProbability(byte[] table, CharSequence keys) {
		int a = START;
		int b = START;
		double total = 0;
		for (int i = 0; i <= keys.length(); i++) {
			int c = i == keys.length() ? END : keys.charAt(i) - 'a';
			total -= (table[(a * SIZE + b) * SIZE + c] & 0xFF) / 16.0;
			a = b;
			b = c;
		}
		return total;
	}

	private static boolean isJamo(int c) {
		return c >= 0x3131 && c <= 0x318E;
	}

	private static InputStream resource(String name) {
		InputStream in = Ranker.class.getResourceAsStream("/assets/jiskorean/" + name);
		if (in == null) {
			throw new IllegalStateException(name + " missing from the mod jar");
		}
		return in;
	}

	/** A word list's words. Generated entries are "word<TAB>count"; only the word is needed. */
	private static Set<String> words(String name) throws IOException {
		Set<String> words = new HashSet<>();
		for (String line : lines(name)) {
			int tab = line.indexOf('\t');
			words.add(tab < 0 ? line : line.substring(0, tab));
		}
		return words;
	}

	/** A bundled text file's lines, without blanks and # comments. Empty if the jar doesn't have it. */
	private static java.util.List<String> lines(String name) throws IOException {
		java.util.List<String> lines = new java.util.ArrayList<>();
		InputStream in = Ranker.class.getResourceAsStream("/assets/jiskorean/" + name);
		if (in == null) {
			return lines;
		}
		try (BufferedReader reader = new BufferedReader(new InputStreamReader(in, StandardCharsets.UTF_8))) {
			for (String line; (line = reader.readLine()) != null; ) {
				line = line.strip();
				if (!line.isEmpty() && !line.startsWith("#")) {
					lines.add(line);
				}
			}
		}
		return lines;
	}
}
