import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Candidates, Engine, Mods, Output } from "./engine";
import { Keyboard } from "./Keyboard";

const REPO = "https://github.com/team-mirae/jis-korean";

function GitHubMark() {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

export function App({ engine }: { engine: Engine }) {
  const [doc, setDoc] = useState("");
  const [preedit, setPreedit] = useState("");
  const [escaped, setEscaped] = useState(false);
  const [shift, setShift] = useState(false);
  const [enterCommits, setEnterCommits] = useState(false);
  const [pressed, setPressed] = useState<Set<string>>(new Set());
  const [focused, setFocused] = useState(false);
  const [candidates, setCandidates] = useState<Candidates | null>(null);
  const [hanjaReady, setHanjaReady] = useState(false);
  const surface = useRef<HTMLDivElement>(null);

  useEffect(() => surface.current?.focus(), []);
  useEffect(() => {
    engine
      .loadDictionary(`${import.meta.env.BASE_URL}hanja.tsv`)
      .then(() => setHanjaReady(true))
      .catch((error) => console.error("hanja dictionary:", error));
  }, [engine]);
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
    setCandidates(out.candidates);
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
    setCandidates(null);
    setEscaped(false);
    surface.current?.focus();
  }

  return (
    <main>
      <header>
        <div className="title">
          <h1>JIS Korean</h1>
          <a className="repo" href={REPO} target="_blank" rel="noreferrer">
            <GitHubMark /> GitHub
          </a>
        </div>
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
          {candidates && <CandidateList candidates={candidates} pageSize={engine.pageSize} onPick={onScreenKey} />}
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
            <dd>Gets an implied ㅇ: ㅏ → 아, 가 + ㅓ → 가어. F6 right after commits the bare vowel.</dd>
          </div>
          <div>
            <dt>F6 한글</dt>
            <dd>Commits. Right after F9/F10, commits the Hangul instead.</dd>
          </div>
          <div>
            <dt>Shift+vowel</dt>
            <dd>Starts a syllable on ㅇ and keeps the final: 말 + ⇧ㅣ → 말이, 오 + ⇧ㅣ → 오이. Pressing ㅣ twice works too.</dd>
          </div>
          <div>
            <dt>F7 한자</dt>
            <dd>{hanjaReady ? "Hanja for the word you just typed. F7 or ↓ for the next, 1–9 to pick, Esc to go back." : "Loading the hanja dictionary…"}</dd>
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
      <footer>
        <a href={REPO} target="_blank" rel="noreferrer">
          Source on GitHub
        </a>
        <span aria-hidden="true">·</span>
        <span>MIT license</span>
      </footer>
    </main>
  );
}

function CandidateList({ candidates, pageSize, onPick }: { candidates: Candidates; pageSize: number; onPick(code: string): void }) {
  const { items, selected } = candidates;
  const start = Math.floor(selected / pageSize) * pageSize;
  const page = items.slice(start, start + pageSize);
  const pages = Math.ceil(items.length / pageSize);
  return (
    <span className="candidates" role="listbox" aria-label="Hanja candidates">
      {page.map((c, i) => (
        <span
          key={start + i}
          role="option"
          aria-selected={start + i === selected}
          className={"candidate" + (start + i === selected ? " selected" : "")}
          onMouseDown={(e) => {
            e.preventDefault();
            onPick(`Digit${i + 1}`);
          }}
        >
          <span className="number">{i + 1}</span>
          <span className="hanja">{c.hanja}</span>
          {c.comment && <span className="comment">{c.comment}</span>}
        </span>
      ))}
      {pages > 1 && (
        <span className="pager">
          {start / pageSize + 1} / {pages} · ← →
        </span>
      )}
    </span>
  );
}
