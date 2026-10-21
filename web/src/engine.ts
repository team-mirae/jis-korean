// The JIS Korean engine, loaded from wasm through its C ABI (engine/include/jis_korean.h).
import wasmUrl from "./engine.wasm?url";

interface Exports {
  memory: WebAssembly.Memory;
  jk_new(): number;
  jk_key(h: number, key: number, mods: number): number;
  jk_flush(h: number): void;
  jk_activate(h: number): void;
  jk_set_enter_commits(h: number, on: number): void;
  jk_is_escaped(h: number): number;
  jk_commit_ptr(h: number): number;
  jk_commit_len(h: number): number;
  jk_preedit_ptr(h: number): number;
  jk_preedit_len(h: number): number;
  jk_key_count(): number;
  jk_key_code_ptr(key: number): number;
  jk_key_code_len(key: number): number;
  jk_legend(key: number, shift: number, escaped: number): number;
  jk_legend_in(key: number, shift: number, escaped: number, mode: number): number;
  jk_set_mode(h: number, mode: number): number;
  jk_set_dedicated_symbols(h: number, on: number): void;
  jk_mode_count(): number;
  jk_mode_id_at(index: number): number;
  jk_mode_name_ptr(mode: number): number;
  jk_mode_name_len(mode: number): number;
  jk_resolve_key_input(h: number, p: number, pl: number): number;
  jk_resolve_key_input_after(h: number, c: number, cl: number, p: number, pl: number): number;
  jk_resolve_keys_for_output(h: number, t: number, tl: number, min: bigint, max: bigint): number;
  jk_resolved_ptr(h: number): number;
  jk_resolved_len(h: number): number;
  jk_alloc(len: number): number;
  jk_dealloc(ptr: number, len: number): void;
  jk_dict_new(ptr: number, len: number): number;
  jk_dict_free(dict: number): void;
  jk_set_dictionary(h: number, dict: number): void;
  jk_candidate_count(h: number): number;
  jk_candidate_selected(h: number): number;
  jk_candidate_page_size(): number;
  jk_candidate_ptr(h: number, i: number): number;
  jk_candidate_len(h: number, i: number): number;
  jk_candidate_comment_ptr(h: number, i: number): number;
  jk_candidate_comment_len(h: number, i: number): number;
  jk_predictor_new(m: number, ml: number, e: number, el: number, k: number, kl: number, n: number, nl: number): number;
  jk_set_predictor(h: number, p: number): void;
  jk_predictor_free(p: number): void;
  jk_predictor_with_words(base: number, k: number, kl: number, e: number, el: number): number;
  jk_learn_example(h: number, t: number, tl: number, keys: number, kl: number): number;
  jk_predict(h: number): number;
  jk_prediction_ptr(h: number, i: number): number;
  jk_prediction_len(h: number, i: number): number;
  jk_prediction_kind(h: number, i: number): number;
  jk_commit_prediction(h: number, i: number): void;
  jk_set_prediction_weight(h: number, which: number, value: number): void;
  jk_set_ranker(h: number, ranker: number): void;
  jk_clear_prediction_weight(h: number, which: number): void;
  jk_set_learning(h: number, on: number): void;
  jk_learned_weight(h: number, which: number): number;
  jk_learner_changed(h: number): number;
  jk_learner_export(h: number): number;
  jk_learner_ptr(h: number): number;
  jk_learner_import(h: number, ptr: number, len: number): number;
  jk_learner_reset(h: number): void;
  jk_plan(h: number, ptr: number, len: number): void;
  jk_plan_jis(h: number): number;
  jk_plan_conventional(h: number): number;
  jk_plan_chars(h: number): number;
  jk_plan_off_keyboard(h: number): number;
  jk_plan_dropped(h: number): number;
}

/** A ranked reading of the staged text (spec §10). */
export interface Prediction {
  text: string;
  kind: PredictionKind;
}

export type PredictionKind =
  | "korean" | "english" | "capitalized" | "upper" | "two-set" | "keep-finals" | "hanja" | "bare" | "mixed" | "implied-l";
const KINDS: PredictionKind[] = [
  "korean", "english", "capitalized", "upper", "two-set", "keep-finals", "hanja", "bare", "mixed", "implied-l",
];

/** Scenarios the user can tune (spec §10.5), in C ABI id order. */
export const WEIGHTS = ["korean", "english", "implied", "mixed", "two-set", "keep-finals", "hanja", "implied-l"] as const;
export type WeightName = (typeof WEIGHTS)[number];

/** How the implied-ㅇ alternatives are ranked (spec §10.7), in C ABI id order. */
export const RANKERS = ["rules", "travel", "route"] as const;
export type RankerName = (typeof RANKERS)[number];

/** Key presses for a text, both ways (Shift counted; symbols off a layout's legends count 3). */
export interface Counts {
  jis: number;
  conventional: number;
  chars: number;
  /** Characters no key types (①, ×, …): 3 presses each on both layouts. */
  off_keyboard: number;
  dropped: number;
}

export interface Mods {
  shift?: boolean;
  ctrl?: boolean;
  alt?: boolean;
  meta?: boolean;
}

export interface Candidate {
  hanja: string;
  comment: string;
}

export interface Candidates {
  items: Candidate[];
  selected: number;
}

export interface Output {
  commit: string;
  preedit: string;
  consumed: boolean;
  /** The hanja list, while one is open (F7). */
  candidates: Candidates | null;
}

const MODIFIER_CODES = new Set([
  "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight",
  "MetaLeft", "MetaRight", "CapsLock", "Fn",
]);

const decoder = new TextDecoder();
const encoder = new TextEncoder();

export class Engine {
  private readonly ids = new Map<string, number>();
  private readonly handle: number;

  constructor(private readonly x: Exports) {
    this.handle = x.jk_new();
    for (let id = 0; id < x.jk_key_count(); id++) {
      this.ids.set(this.text(x.jk_key_code_ptr(id), x.jk_key_code_len(id)), id);
    }
  }

  private text(ptr: number, len: number): string {
    return decoder.decode(new Uint8Array(this.x.memory.buffer, ptr, len));
  }

  private id(code: string): number {
    if (MODIFIER_CODES.has(code)) return this.ids.get("Modifier")!;
    return this.ids.get(code) ?? this.ids.get("Other")!;
  }

  private output(consumed: boolean): Output {
    const { x, handle: h } = this;
    const count = x.jk_candidate_count(h);
    const items: Candidate[] = [];
    for (let i = 0; i < count; i++) {
      items.push({
        hanja: this.text(x.jk_candidate_ptr(h, i), x.jk_candidate_len(h, i)),
        comment: this.text(x.jk_candidate_comment_ptr(h, i), x.jk_candidate_comment_len(h, i)),
      });
    }
    return {
      commit: this.text(x.jk_commit_ptr(h), x.jk_commit_len(h)),
      preedit: this.text(x.jk_preedit_ptr(h), x.jk_preedit_len(h)),
      consumed,
      candidates: count ? { items, selected: x.jk_candidate_selected(h) } : null,
    };
  }

  get pageSize(): number {
    return this.x.jk_candidate_page_size();
  }

  /** Loads the hanja dictionary (dict/hanja.tsv) so F7 works. */
  async loadDictionary(url: string): Promise<void> {
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    const { x } = this;
    const ptr = x.jk_alloc(bytes.length);
    new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
    const dict = x.jk_dict_new(ptr, bytes.length);
    x.jk_dealloc(ptr, bytes.length);
    if (!dict) throw new Error("hanja dictionary is not UTF-8");
    x.jk_set_dictionary(this.handle, dict);
    // The engine keeps its own reference.
    x.jk_dict_free(dict);
  }

  private predictor = 0;
  /** The predictor as loaded; `predictor` may be it with the user's own words (training file). */
  private basePredictor = 0;
  private predictionsOn = false;
  private ranker: RankerName = "rules";

  /** Loads the predictive layer (ranker model and word lists). Turn it on with `setPredictions`. */
  async loadPredictor(base: string): Promise<void> {
    const get = async (file: string) => new Uint8Array(await (await fetch(base + file)).arrayBuffer());
    const [model, english, korean, nouns] = await Promise.all([
      get("ranker.bin"),
      get("common-en.txt"),
      get("common-ko.txt"),
      get("english-nouns.txt"),
    ]);
    const { x } = this;
    const put = (bytes: Uint8Array) => {
      const ptr = x.jk_alloc(bytes.length);
      new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
      return ptr;
    };
    const [m, e, k, n] = [put(model), put(english), put(korean), put(nouns)];
    this.predictor = x.jk_predictor_new(m, model.length, e, english.length, k, korean.length, n, nouns.length);
    x.jk_dealloc(m, model.length);
    x.jk_dealloc(e, english.length);
    x.jk_dealloc(k, korean.length);
    x.jk_dealloc(n, nouns.length);
    if (!this.predictor) throw new Error("predictor model is malformed");
    this.basePredictor = this.predictor;
  }

  /** On: the engine ranks readings of the staged text. Off: the deterministic lite engine. */
  setPredictions(on: boolean): void {
    this.predictionsOn = on;
    this.x.jk_set_predictor(this.handle, on ? this.predictor : 0);
  }

  /** Adds the user's own words (training contract): lines of `word` or `word<TAB>count`. Replaces
   * any added before. */
  setUserWords(korean: string, english: string): void {
    const { x } = this;
    const k = encoder.encode(korean);
    const e = encoder.encode(english);
    const kp = x.jk_alloc(k.length);
    const ep = x.jk_alloc(e.length);
    new Uint8Array(x.memory.buffer, kp, k.length).set(k);
    new Uint8Array(x.memory.buffer, ep, e.length).set(e);
    const next = x.jk_predictor_with_words(this.basePredictor, kp, k.length, ep, e.length);
    x.jk_dealloc(kp, k.length);
    x.jk_dealloc(ep, e.length);
    if (!next) throw new Error("words are not UTF-8");
    this.usePredictor(next);
  }

  /** Back to the built-in word lists. */
  clearUserWords(): void {
    this.usePredictor(this.basePredictor);
  }

  private usePredictor(next: number): void {
    if (this.predictor !== this.basePredictor) this.x.jk_predictor_free(this.predictor);
    this.predictor = next;
    if (this.predictionsOn) this.x.jk_set_predictor(this.handle, next);
  }

  /** Learns from an example of what the user meant (training contract): `keys` as
   * KeyboardEvent.code names, "S-" for Shift, or none for the cheapest keys. Returns where the text
   * was listed (0 = top), -1 if it wasn't, -2 if the input was malformed. */
  learnExample(text: string, keys?: string): number {
    const { x } = this;
    const t = encoder.encode(text);
    const k = encoder.encode(keys ?? "");
    const tp = x.jk_alloc(t.length);
    const kp = x.jk_alloc(Math.max(k.length, 1));
    new Uint8Array(x.memory.buffer, tp, t.length).set(t);
    new Uint8Array(x.memory.buffer, kp, k.length).set(k);
    const at = x.jk_learn_example(this.handle, tp, t.length, kp, k.length);
    x.jk_dealloc(tp, t.length);
    x.jk_dealloc(kp, Math.max(k.length, 1));
    return at;
  }

  /** The ranked readings of the staged text (empty when predictions are off). */
  predictions(): Prediction[] {
    const { x, handle: h } = this;
    const count = x.jk_predict(h);
    const out: Prediction[] = [];
    for (let i = 0; i < count; i++) {
      out.push({ text: this.text(x.jk_prediction_ptr(h, i), x.jk_prediction_len(h, i)), kind: KINDS[x.jk_prediction_kind(h, i)] });
    }
    return out;
  }

  /** Chooses how the implied-ㅇ alternatives are ranked. */
  setRanker(ranker: RankerName): void {
    this.ranker = ranker;
    this.x.jk_set_ranker(this.handle, RANKERS.indexOf(ranker));
  }

  /** `predictions()` as `ranker` would rank them, leaving the engine's own ranker as it was. */
  predictionsBy(ranker: RankerName): Prediction[] {
    return this.withRanker(ranker, () => this.predictions());
  }

  /** `commitPrediction()` from `ranker`'s list. */
  commitPredictionBy(ranker: RankerName, index: number): Output {
    return this.withRanker(ranker, () => this.commitPrediction(index));
  }

  private withRanker<T>(ranker: RankerName, run: () => T): T {
    const own = this.ranker;
    this.x.jk_set_ranker(this.handle, RANKERS.indexOf(ranker));
    try {
      return run();
    } finally {
      this.x.jk_set_ranker(this.handle, RANKERS.indexOf(own));
    }
  }

  /** Tunes one scenario: a multiplier on its learned probability (1 = as learned; 0 turns an
   * alternative off). */
  setWeight(which: WeightName, value: number): void {
    this.x.jk_set_prediction_weight(this.handle, WEIGHTS.indexOf(which), value);
  }

  /** Hands a scenario's weight back to learning (or 1 when learning is off). */
  clearWeight(which: WeightName): void {
    this.x.jk_clear_prediction_weight(this.handle, WEIGHTS.indexOf(which));
  }

  /** Learning from the user's choices (spec §10.11). The engine records choices itself. */
  setLearning(on: boolean): void {
    this.x.jk_set_learning(this.handle, on ? 1 : 0);
  }

  /** The learned multiplier for a scenario (0.5–2; 1 = no change). */
  learnedWeight(which: WeightName): number {
    return this.x.jk_learned_weight(this.handle, WEIGHTS.indexOf(which));
  }

  /** Whether what was learned changed since the last call. */
  learnerChanged(): boolean {
    return this.x.jk_learner_changed(this.handle) !== 0;
  }

  /** What was learned, as one line of counts (no text, no keys). */
  exportLearner(): string {
    const len = this.x.jk_learner_export(this.handle);
    return this.text(this.x.jk_learner_ptr(this.handle), len);
  }

  /** Restores an export; false if it wasn't one. */
  importLearner(state: string): boolean {
    const { x } = this;
    const bytes = encoder.encode(state);
    const ptr = x.jk_alloc(bytes.length);
    new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
    const ok = x.jk_learner_import(this.handle, ptr, bytes.length) !== 0;
    x.jk_dealloc(ptr, bytes.length);
    return ok;
  }

  resetLearner(): void {
    this.x.jk_learner_reset(this.handle);
  }

  /** Commits reading `index` of `predictions()`. */
  commitPrediction(index: number): Output {
    this.x.jk_commit_prediction(this.handle, index);
    return this.output(true);
  }

  /** Counts the key presses `text` takes on JIS Korean and on conventional 2-set. */
  plan(text: string): Counts {
    const { x, handle: h } = this;
    const bytes = encoder.encode(text);
    const ptr = x.jk_alloc(bytes.length);
    new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
    x.jk_plan(h, ptr, bytes.length);
    x.jk_dealloc(ptr, bytes.length);
    return {
      jis: x.jk_plan_jis(h),
      conventional: x.jk_plan_conventional(h),
      chars: x.jk_plan_chars(h),
      off_keyboard: x.jk_plan_off_keyboard(h),
      dropped: x.jk_plan_dropped(h),
    };
  }

  /** The modes (spec §3.6): id and name. Mode A (0) is the default. */
  modes(): { id: number; name: string }[] {
    const { x } = this;
    // Ids have gaps (spec §3.6.1): list them by position, never 0..count.
    return Array.from({ length: x.jk_mode_count() }, (_, index) => {
      const id = x.jk_mode_id_at(index);
      return { id, name: this.text(x.jk_mode_name_ptr(id), x.jk_mode_name_len(id)) };
    });
  }

  /** Switches the mode: 0 = Mode A (JIS), 1 = Mode B (US ANSI), 2 = Mode C (UK ISO). */
  setMode(id: number): void {
    // An id this build doesn't have is refused and the mode stays as it was.
    if (this.x.jk_set_mode(this.handle, id)) this.mode = id;
  }

  /**
   * Dedicated symbol keys (spec §3.6), on by default. Off, Mode C's [ ] and ISO extra key type
   * their UK ISO characters.
   */
  setDedicatedSymbols(on: boolean): void {
    this.symbols = on;
    this.x.jk_set_dedicated_symbols(this.handle, on ? 1 : 0);
  }

  private mode = 0;
  private symbols = true;

  /** Copies `text` into the engine's memory for one call. */
  private withBytes<T>(texts: string[], f: (args: number[]) => T): T {
    const { x } = this;
    const args: number[] = [];
    const held: [number, number][] = [];
    for (const text of texts) {
      const bytes = encoder.encode(text);
      const ptr = x.jk_alloc(bytes.length);
      new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
      args.push(ptr, bytes.length);
      held.push([ptr, bytes.length]);
    }
    try {
      return f(args);
    } finally {
      for (const [ptr, len] of held) x.jk_dealloc(ptr, len);
    }
  }

  private resolved(): string {
    return this.text(this.x.jk_resolved_ptr(this.handle), this.x.jk_resolved_len(this.handle));
  }

  /**
   * resolveKeyCodeInput (spec §8.3): what a key press ("KeyG", "S-Digit2") types on its own, or,
   * with `context`, what the field shows after that unconfirmed text and the key. Null if none.
   * Runs on a copy of this engine: its mode and predictions.
   */
  resolveKeyCodeInput(press: string, context?: string): string | null {
    const { x, handle: h } = this;
    const found =
      context === undefined
        ? this.withBytes([press], ([p, pl]) => x.jk_resolve_key_input(h, p, pl))
        : this.withBytes([context, press], ([c, cl, p, pl]) => x.jk_resolve_key_input_after(h, c, cl, p, pl));
    return found ? this.resolved() : null;
  }

  /**
   * resolveKeyCodeFromOutput (spec §8.3): every combination of key presses that types `text`, with
   * `min` to `max` presses. Empty for a nonsensical range.
   */
  resolveKeyCodeFromOutput(text: string, min = 1, max = 1): string[][] {
    const { x, handle: h } = this;
    const clamp = (n: number) => BigInt(Math.trunc(Math.max(-1e15, Math.min(1e15, Number.isFinite(n) ? n : 0))));
    this.withBytes([text], ([t, tl]) => x.jk_resolve_keys_for_output(h, t, tl, clamp(min), clamp(max)));
    return JSON.parse(this.resolved()) as string[][];
  }

  key(code: string, mods: Mods = {}): Output {
    const bits = (mods.shift ? 1 : 0) | (mods.ctrl ? 2 : 0) | (mods.alt ? 4 : 0) | (mods.meta ? 8 : 0);
    return this.output(this.x.jk_key(this.handle, this.id(code), bits) !== 0);
  }

  flush(): Output {
    this.x.jk_flush(this.handle);
    return this.output(true);
  }

  activate(): void {
    this.x.jk_activate(this.handle);
  }

  setEnterCommits(on: boolean): void {
    this.x.jk_set_enter_commits(this.handle, on ? 1 : 0);
  }

  get escaped(): boolean {
    return this.x.jk_is_escaped(this.handle) !== 0;
  }

  /** The character a key types, for key legends. Empty if none. */
  legend(code: string, shift: boolean, escaped: boolean): string {
    // With Dedicated symbol keys off, Mode C's optional keys type their stock (F8) characters.
    const stock = escaped || (!this.symbols && this.mode === 2 && OPTIONAL_KEYS.has(code));
    const c = this.x.jk_legend_in(this.id(code), shift ? 1 : 0, stock ? 1 : 0, this.mode);
    return c ? String.fromCodePoint(c) : "";
  }
}

/** Mode C's keys that Dedicated symbol keys turns off (the engine's Mode::optional_keys). */
const OPTIONAL_KEYS = new Set(["BracketLeft", "BracketRight", "IntlBackslash"]);

export async function loadEngine(): Promise<Engine> {
  const { instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl));
  return new Engine(instance.exports as unknown as Exports);
}
