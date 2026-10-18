import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Engine, Mods, Output } from "./engine";
import { Keyboard } from "./Keyboard";

export function App({ engine }: { engine: Engine }) {
  const [doc, setDoc] = useState("");
  const [preedit, setPreedit] = useState("");
  const [escaped, setEscaped] = useState(false);
  const [shift, setShift] = useState(false);
  const [enterCommits, setEnterCommits] = useState(false);
  const [pressed, setPressed] = useState<Set<string>>(new Set());
  const [focused, setFocused] = useState(false);
  const surface = useRef<HTMLDivElement>(null);

  useEffect(() => surface.current?.focus(), []);
  useEffect(() => {
    surface.current?.scrollTo({ top: surface.current.scrollHeight });
  }, [doc, preedit]);

  // Applies an engine result, then what a passed-through key does in a plain text field.
  function apply(out: Output, code: string, mods: Mods) {
    setDoc((d) => {
      let next = d + out.commit;
      if (!out.consumed && !(mods.ctrl || mods.alt || mods.meta)) {
        if (code === "Enter") next += "\n";
        else if (code === "Backspace") next = [...next].slice(0, -1).join("");
      }
      return next;
    });
    setPreedit(out.preedit);
    setEscaped(engine.escaped);
  }

  function type(code: string, mods: Mods): Output {
    const out = engine.key(code, mods);
    apply(out, code, mods);
    return out;
  }

  function onKeyDown(e: ReactKeyboardEvent) {
    // An OS input method is composing: these keys aren't ours.
    if (e.nativeEvent.isComposing || e.keyCode === 229) return;
    setPressed((p) => new Set(p).add(e.code));
    const out = type(e.code, { shift: e.shiftKey || shift, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey });
    if (!e.code.startsWith("Shift")) setShift(false);
    if (out.consumed || e.code === "Enter" || e.code === "Backspace") e.preventDefault();
  }

  function onKeyUp(e: ReactKeyboardEvent) {
    setPressed((p) => {
      const next = new Set(p);
      next.delete(e.code);
      return next;
    });
  }

  function onScreenKey(code: string) {
    type(code, { shift });
    setShift(false);
    surface.current?.focus();
  }

  function onBlur() {
    // Focus loss commits, as in a real input method.
    apply(engine.flush(), "", {});
    setPressed(new Set());
    setFocused(false);
  }

  function clear() {
    engine.flush();
    engine.activate();
    setDoc("");
    setPreedit("");
    setEscaped(false);
    surface.current?.focus();
  }

  return (
    <main>
      <header>
        <h1>JIS Korean</h1>
        <p className="lede">
          Korean 2-set (두벌식) for JIS keyboards: double consonants and compound vowels on the number row, Korean
          punctuation on the symbol keys, and Japanese-IME-style F6–F10. Try it here before installing.
        </p>
      </header>

      <section className="editor" aria-label="Try it">
        <div className="status">
          <span className={"chip" + (escaped ? " escaped" : "")}>
            {escaped ? "Standard 2-set · Shift+F8 returns" : "JIS Korean"}
          </span>
          <span className="hint">{focused ? "Use an English input source on your OS." : "Click the box to type."}</span>
        </div>
        <div
          ref={surface}
          className="surface"
          role="textbox"
          aria-multiline="true"
          aria-label="Typing area"
          tabIndex={0}
          onKeyDown={onKeyDown}
          onKeyUp={onKeyUp}
          onFocus={() => setFocused(true)}
          onBlur={onBlur}
        >
          {doc}
          {preedit && <span className="preedit">{preedit}</span>}
          {focused && <span className="caret" aria-hidden="true" />}
          {!doc && !preedit && !focused && <span className="placeholder">한글을 입력해 보세요</span>}
        </div>
        <div className="controls">
          <button
            type="button"
            className="toggle"
            aria-pressed={shift}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => setShift(!shift)}
          >
            Shift
          </button>
          <label className="check">
            <input
              type="checkbox"
              checked={enterCommits}
              onChange={(e) => {
                setEnterCommits(e.target.checked);
                engine.setEnterCommits(e.target.checked);
              }}
            />
            Enter commits without a new line
          </label>
          <span className="spacer" />
          <button type="button" onClick={() => navigator.clipboard?.writeText(doc + preedit)}>
            Copy
          </button>
          <button type="button" onClick={clear}>
            Clear
          </button>
        </div>
      </section>

      <Keyboard engine={engine} escaped={escaped} shift={shift} pressed={pressed} onPress={onScreenKey} />

      <section className="reference">
        <h2>Keys to try</h2>
        <dl>
          <div>
            <dt>1–5</dt>
            <dd>ㅃ ㅉ ㄸ ㄲ ㅆ. Shift+Q to Shift+T still work.</dd>
          </div>
          <div>
            <dt>6–0</dt>
            <dd>ㅟ ㅝ ㅚ ㅢ ㅘ. ㅗ then ㅏ still makes ㅘ. Shift types the digit.</dd>
          </div>
          <div>
            <dt>Lone vowel</dt>
            <dd>Gets an implied ㅇ: ㅏ → 아, 가 + ㅓ → 가어. F6 right after leaves the bare vowel.</dd>
          </div>
          <div>
            <dt>F6 한글</dt>
            <dd>Ends the syllable being composed (한 F6 ㅏ → 한아), or undoes F9/F10.</dd>
          </div>
          <div>
            <dt>F8 두벌식</dt>
            <dd>Standard 2-set with the stock JIS symbols, until Shift+F8.</dd>
          </div>
          <div>
            <dt>F9 / F10</dt>
            <dd>The word as full- or half-width letters. Press again to cycle case.</dd>
          </div>
          <div>
            <dt>Space</dt>
            <dd>Commits and types a space. Shift+Enter commits only.</dd>
          </div>
        </dl>
      </section>
    </main>
  );
}
