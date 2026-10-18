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
}

export interface Mods {
  shift?: boolean;
  ctrl?: boolean;
  alt?: boolean;
  meta?: boolean;
}

export interface Output {
  commit: string;
  preedit: string;
  consumed: boolean;
}

const MODIFIER_CODES = new Set([
  "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight",
  "MetaLeft", "MetaRight", "CapsLock", "Fn",
]);

const decoder = new TextDecoder();

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
    return {
      commit: this.text(x.jk_commit_ptr(h), x.jk_commit_len(h)),
      preedit: this.text(x.jk_preedit_ptr(h), x.jk_preedit_len(h)),
      consumed,
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
