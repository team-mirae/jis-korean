// The sample text as a typing guide: while what's typed follows it, the rest stays visible.

const INITIALS = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const MEDIALS = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const FINALS = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";
/** Compound jamo, split into what 2-set types for them. */
const SPLIT: Record<string, string> = {
  ㅘ: "ㅗㅏ", ㅙ: "ㅗㅐ", ㅚ: "ㅗㅣ", ㅝ: "ㅜㅓ", ㅞ: "ㅜㅔ", ㅟ: "ㅜㅣ", ㅢ: "ㅡㅣ",
  ㄳ: "ㄱㅅ", ㄵ: "ㄴㅈ", ㄶ: "ㄴㅎ", ㄺ: "ㄹㄱ", ㄻ: "ㄹㅁ", ㄼ: "ㄹㅂ", ㄽ: "ㄹㅅ", ㄾ: "ㄹㅌ",
  ㄿ: "ㄹㅍ", ㅀ: "ㄹㅎ", ㅄ: "ㅂㅅ",
};

/** A character as the jamo that make it up (ㅘ and ㄺ split), or itself. */
function jamo(ch: string): string {
  const n = ch.charCodeAt(0) - 0xac00;
  if (n < 0 || n >= 11172) return SPLIT[ch] ?? ch;
  const parts = [INITIALS[Math.floor(n / 588)], MEDIALS[Math.floor((n % 588) / 28)], FINALS[n % 28].trim()];
  return parts.map((p) => SPLIT[p] ?? p).join("");
}

/**
 * What's left of `sample` to type, if `typed` follows it so far; "" once it doesn't. The syllable
 * being composed only has to match as far as it goes, and may hold a final that the next sample
 * syllable will take (typing 아나 shows 안 for a moment).
 */
export function ghost(sample: string, typed: string): string {
  if (!typed) return sample;
  if (sample.startsWith(typed)) return sample.slice(typed.length);
  const head = typed.slice(0, -1);
  if (!sample.startsWith(head)) return "";
  const rest = sample.slice(head.length);
  const ahead = [...rest.slice(0, 3)].map(jamo).join("");
  return ahead.startsWith(jamo(typed.slice(-1))) ? rest.slice(1) : "";
}
