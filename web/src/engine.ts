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
  jk_plan(h: number, ptr: number, len: number): void;
  jk_plan_jis(h: number): number;
  jk_plan_conventional(h: number): number;
  jk_plan_chars(h: number): number;
  jk_plan_off_keyboard(h: number): number;
  jk_plan_dropped(h: number): number;
}

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
    const c = this.x.jk_legend(this.id(code), shift ? 1 : 0, escaped ? 1 : 0);
    return c ? String.fromCodePoint(c) : "";
  }
}

export async function loadEngine(): Promise<Engine> {
  const { instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl));
  return new Engine(instance.exports as unknown as Exports);
}
