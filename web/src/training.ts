// The training contract (docs/training-contract.md): read a training file and apply it.
import type { Engine, WeightName } from "./engine";
import { WEIGHTS } from "./engine";

export interface TrainingWord {
  word: string;
  count?: number;
  confidence?: "tentative" | "likely" | "confirmed";
}

export interface TrainingFile {
  format: "jis-korean-training";
  version: 1;
  source?: { tool?: string; created?: string; description?: string };
  weights?: Partial<Record<WeightName, number>>;
  vocabulary?: { korean?: TrainingWord[]; english?: TrainingWord[] };
  examples?: { text: string; keys?: string; count?: number }[];
}

/** What applying a file did, for the summary. */
export interface TrainingSummary {
  source: string;
  weights: number;
  words: number;
  examples: { top: number; listed: number; missed: string[] };
}

const CONFIDENCE_COUNT = { tentative: 10, likely: 20, confirmed: 50 } as const;
const KEYS = /^((S-)?(Key[A-Z]|Digit[0-9]|Minus|Equal|IntlYen|BracketLeft|BracketRight|Semicolon|Quote|Backslash|Comma|Period|Slash|IntlRo|F6|F7|F8|F9|F10)( |$))+$/;

/** Checks a parsed file against the contract's rules; returns the problems, empty when it's fine. */
export function check(file: unknown): string[] {
  const problems: string[] = [];
  const f = file as Partial<TrainingFile> | null;
  if (!f || typeof f !== "object") return ["not a JSON object"];
  if (f.format !== "jis-korean-training") problems.push('format must be "jis-korean-training"');
  if (f.version !== 1) problems.push("version must be 1");
  for (const [k, v] of Object.entries(f.weights ?? {})) {
    if (!(WEIGHTS as readonly string[]).includes(k)) problems.push(`unknown weight "${k}"`);
    else if (typeof v !== "number" || v < 0 || v > 10) problems.push(`weight "${k}" must be 0–10`);
  }
  const words = (list: TrainingWord[] | undefined, pattern: RegExp, name: string) => {
    if (list === undefined) return;
    if (!Array.isArray(list) || list.length > 5000) return problems.push(`vocabulary.${name}: a list of at most 5,000 words`);
    for (const w of list) if (!w || typeof w.word !== "string" || !pattern.test(w.word)) problems.push(`vocabulary.${name}: bad word ${JSON.stringify(w?.word)}`);
  };
  words(f.vocabulary?.korean, /^[가-힣]+$/, "korean");
  words(f.vocabulary?.english, /^[A-Za-z]+$/, "english");
  if (f.examples !== undefined) {
    if (!Array.isArray(f.examples) || f.examples.length > 2000) problems.push("examples: a list of at most 2,000");
    else
      for (const e of f.examples) {
        if (!e || typeof e.text !== "string" || !/^\S{1,60}$/.test(e.text)) problems.push(`examples: bad text ${JSON.stringify(e?.text)}`);
        if (e?.keys !== undefined && (typeof e.keys !== "string" || !KEYS.test(e.keys))) problems.push(`examples: bad keys for ${e.text}`);
      }
  }
  return problems.slice(0, 8);
}

/** The vocabulary as the engine's word lists (`word<TAB>count` lines). */
export function wordLists(file: TrainingFile): { korean: string; english: string } {
  const lines = (list: TrainingWord[] = []) =>
    list.map((w) => `${w.word}\t${w.count ?? (w.confidence ? CONFIDENCE_COUNT[w.confidence] : 1000)}`).join("\n");
  return { korean: lines(file.vocabulary?.korean), english: lines(file.vocabulary?.english) };
}

/** Learns from the file's examples; the weights and words are applied by the caller (they're
 * stored with the page's settings). */
export function learnExamples(engine: Engine, file: TrainingFile): TrainingSummary["examples"] {
  const result = { top: 0, listed: 0, missed: [] as string[] };
  for (const e of file.examples ?? []) {
    let at = -1;
    for (let i = 0; i < Math.min(Math.max(e.count ?? 1, 1), 20); i++) at = engine.learnExample(e.text, e.keys);
    if (at === 0) result.top++;
    else if (at > 0) result.listed++;
    else result.missed.push(e.text);
  }
  return result;
}
