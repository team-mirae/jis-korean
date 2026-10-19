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
	/** Canonical 2-set keys for the dedicated number-row jamo (1–5 ㅃ ㅉ ㄸ ㄲ ㅆ, 6–0 ㅟ ㅝ ㅚ ㅢ ㅘ). */
	private static final String[] DIGIT_KEYS = {"hk", "q", "w", "e", "r", "t", "nl", "nj", "hl", "ml"};

	static final double LEXICON = 2.0;
	static final double STICKY = 0.5;
	static final double BARE_JAMO = 1.5;

	private final byte[] english;
	private final byte[] korean;
	private final Set<String> englishWords;
	private final Set<String> koreanWords;

	Ranker(byte[] english, byte[] korean, Set<String> englishWords, Set<String> koreanWords) {
		this.english = english;
		this.korean = korean;
		this.englishWords = englishWords;
		this.koreanWords = koreanWords;
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
			return new Ranker(en, ko, words("common-en.txt"), words("common-ko.txt"));
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
		for (char c : englishText.toCharArray()) {
			if (c >= 'A' && c <= 'Z' && KOREAN_SHIFTS.indexOf(c) < 0) {
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
			return new Choice(Language.ENGLISH, -99, "command");
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

	private static Set<String> words(String name) throws IOException {
		Set<String> words = new HashSet<>();
		try (BufferedReader reader = new BufferedReader(new InputStreamReader(resource(name), StandardCharsets.UTF_8))) {
			for (String line; (line = reader.readLine()) != null; ) {
				line = line.strip();
				if (!line.isEmpty() && !line.startsWith("#")) {
					words.add(line);
				}
			}
		}
		return words;
	}
}
