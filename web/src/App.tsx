import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Candidates, Counts, Engine, Mods, Output, Prediction, PredictionKind } from "./engine";
import { ghost } from "./ghost";
import { Keyboard } from "./Keyboard";
import benchmarks from "./stats.json";

const REPO = "https://github.com/team-mirae/jis-korean";

/** Practice texts in public/: the gray guide in the typing area, and what's counted when it's empty. */
const SAMPLES = [
  { id: "short", file: "sample.txt", label: "Short" },
  { id: "long", file: "long-sample-text.txt", label: "AI since ChatGPT (long)" },
];

/** The keyboard floats over the page once the typing area is taller than this many lines. */
const FLOAT_AFTER_LINES = 15;

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
  const [sampleId, setSampleId] = useState(SAMPLES[0].id);
  const [samples, setSamples] = useState<Record<string, string>>({});
  const sample = samples[sampleId] ?? "한글을 입력해 보세요";
  const [counts, setCounts] = useState<Counts | null>(null);
  const [predictionsOn, setPredictionsOn] = useState(() => loadSetting("predictions", true));
  const [readings, setReadings] = useState<Prediction[]>([]);
  const [floating, setFloating] = useState(false);
  const surface = useRef<HTMLDivElement>(null);
  const caret = useRef<HTMLSpanElement>(null);

  useEffect(() => surface.current?.focus(), []);
  useEffect(() => {
    // Practice texts from public/, editable without a rebuild.
    for (const { id, file } of SAMPLES) {
      fetch(`${import.meta.env.BASE_URL}${file}`)
        .then((response) => (response.ok ? response.text() : Promise.reject(response.status)))
        .then((text) => setSamples((s) => ({ ...s, [id]: text.trim() })))
        .catch(() => {});
    }
  }, []);
  useEffect(() => {
    // Past FLOAT_AFTER_LINES lines, the keyboard floats so it stays in view while typing.
    const box = surface.current;
    if (!box) return;
    const observer = new ResizeObserver(() => {
      const style = getComputedStyle(box);
      const lines = (box.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom)) / parseFloat(style.lineHeight);
      setFloating(lines > FLOAT_AFTER_LINES);
    });
    observer.observe(box);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    engine
      .loadPredictor(`${import.meta.env.BASE_URL}predict/`)
      .then(() => engine.setPredictions(predictionsOn))
      .catch((error) => console.error("predictor:", error));
    // Only once: later changes go through togglePredictions.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [engine]);
  useEffect(() => {
    engine
      .loadDictionary(`${import.meta.env.BASE_URL}hanja.tsv`)
      .then(() => setHanjaReady(true))
      .catch((error) => console.error("hanja dictionary:", error));
  }, [engine]);
  useEffect(() => {
    // Keep the caret in view (and clear of a floating keyboard) as the text grows.
    caret.current?.scrollIntoView({ block: "nearest" });
  }, [doc, preedit]);

  // Applies an engine result, then what a passed-through key does in a plain text field.
  // Key presses for what's in the box (or the sample when it's empty), once typing pauses.
  const counted = doc + preedit || sample;
  // The sample stays visible past the caret while what's typed follows it.
  const rest = ghost(sample, doc + preedit);
  useEffect(() => {
    const timer = setTimeout(() => setCounts(engine.plan(counted)), 150);
    return () => clearTimeout(timer);
  }, [engine, counted]);

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
    // The predictive layer's readings of what's staged (none while a hanja list is open).
    setReadings(out.preedit && !out.candidates ? engine.predictions() : []);
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
    // With predictions on, Space commits the top reading (and types the space).
    if (e.code === "Space" && readings.length && !(e.ctrlKey || e.altKey || e.metaKey)) {
      e.preventDefault();
      pick(0, " ");
      return;
    }
    const out = type(e.code, { shift: e.shiftKey || shift, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey });
    if (!e.code.startsWith("Shift")) setShift(false);
    if (out.consumed || e.code === "Enter" || e.code === "Backspace") e.preventDefault();
  }

  /** Commits reading `index` of the predictions, then `suffix`. */
  function pick(index: number, suffix = "") {
    const out = engine.commitPrediction(index);
    apply({ ...out, commit: out.commit + suffix }, "", {});
  }

  function togglePredictions(on: boolean) {
    setPredictionsOn(on);
    saveSetting("predictions", on);
    engine.setPredictions(on);
    setReadings(on && preedit ? engine.predictions() : []);
    surface.current?.focus();
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

  function chooseSample(id: string) {
    // A new practice text starts from an empty box.
    clear();
    setSampleId(id);
  }

  return (
    <main className={floating ? "floating-keyboard" : undefined}>
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
          <span className="spacer" />
          <span className="samples" role="group" aria-label="Practice text">
            {SAMPLES.map(({ id, label }) => (
              <button
                key={id}
                type="button"
                aria-pressed={sampleId === id}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => chooseSample(id)}
              >
                {label}
              </button>
            ))}
          </span>
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
          {focused && <span ref={caret} className="caret" aria-hidden="true" />}
          {candidates && <CandidateList candidates={candidates} pageSize={engine.pageSize} onPick={onScreenKey} />}
          {!candidates && readings.length > 0 && <PredictionList readings={readings} onPick={(i) => pick(i)} />}
          {rest && <span className="placeholder">{rest}</span>}
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
            <input type="checkbox" checked={predictionsOn} onChange={(e) => togglePredictions(e.target.checked)} />
            Predictions
          </label>
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

      <Keyboard
        engine={engine}
        escaped={escaped}
        shift={shift}
        pressed={pressed}
        floating={floating}
        onPress={onScreenKey}
      />

      {counts && <KeyCounts counts={counts} sample={!doc && !preedit} />}

      <Benchmarks />

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
          <div>
            <dt>Predictions</dt>
            <dd>On by default: the likeliest reading of what you typed (Korean, English, plain 2-set, …) is highlighted, and Space commits it. Click another to use it instead. Turn off for plain JIS Korean.</dd>
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

function KeyCounts({ counts, sample }: { counts: Counts; sample: boolean }) {
  const saved = counts.conventional - counts.jis;
  const percent = counts.conventional ? Math.round((saved / counts.conventional) * 1000) / 10 : 0;
  const format = (n: number) => n.toLocaleString("en-US");
  return (
    <section className="counts" aria-label="Key presses" aria-live="polite">
      <span className="counts-label">{sample ? "Key presses for the sample text" : "Key presses for your text"}</span>
      <span className="count">
        <span className="count-name">Conventional 2-set</span>
        <strong>{format(counts.conventional)}</strong>
      </span>
      <span className="count jis">
        <span className="count-name">JIS Korean</span>
        <strong>{format(counts.jis)}</strong>
      </span>
      <span className="count saved">
        <span className="count-name">{saved >= 0 ? "Fewer" : "More"}</span>
        <strong>{format(Math.abs(saved))} ({Math.abs(percent)}%)</strong>
      </span>
      <span className="counts-note">
        {format(counts.chars)} characters. Shift counts as a press; a symbol a layout has no key for counts as 3.
        {counts.dropped > 0 && ` ${format(counts.dropped)} untypable characters left out.`}
      </span>
    </section>
  );
}

/** Precomputed with the same planner by tools/build_stats.py (texts from Korean Wikisource). */
function Benchmarks() {
  const format = (n: number) => n.toLocaleString("en-US");
  return (
    <section className="benchmarks" aria-label="Benchmarks">
      <h2>Key presses for well-known texts</h2>
      <table>
        <thead>
          <tr>
            <th scope="col">Text</th>
            <th scope="col" className="number">Characters</th>
            <th scope="col" className="number">Conventional 2-set</th>
            <th scope="col" className="number">JIS Korean</th>
            <th scope="col" className="number">Fewer</th>
          </tr>
        </thead>
        <tbody>
          {benchmarks.map((b) => {
            const saved = b.conventional - b.jis;
            return (
              <tr key={b.id}>
                <th scope="row">
                  <a href={b.source} target="_blank" rel="noreferrer">
                    {b.title}
                  </a>
                  <span className="subtitle">{b.subtitle}</span>
                </th>
                <td className="number">{format(b.chars)}</td>
                <td className="number">{format(b.conventional)}</td>
                <td className="number jis">{format(b.jis)}</td>
                <td className="number jis">
                  {format(saved)} ({Math.round((saved / b.conventional) * 1000) / 10}%)
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <p className="counts-note">
        Same rules as above: Shift counts as a press, and a character neither layout has a key for (such as the
        circled paragraph numbers ①②③ in the laws) counts as 3 on both. Texts from Korean Wikisource; each title
        links to the exact revision.
      </p>
    </section>
  );
}

const KIND_LABELS: Record<PredictionKind, string> = {
  korean: "한",
  english: "En",
  capitalized: "Aa",
  upper: "AA",
  "two-set": "두벌식",
  "keep-finals": "받침",
  hanja: "漢",
};

/** The predictive layer's ranked readings of the staged text, top one first. */
function PredictionList({ readings, onPick }: { readings: Prediction[]; onPick(index: number): void }) {
  return (
    <span className="predictions" role="listbox" aria-label="Predictions">
      {readings.map((r, i) => (
        <span
          key={r.kind + r.text}
          role="option"
          aria-selected={i === 0}
          className={"prediction" + (i === 0 ? " top" : "")}
          onMouseDown={(e) => {
            e.preventDefault();
            onPick(i);
          }}
        >
          <span className="kind">{KIND_LABELS[r.kind]}</span>
          {r.text}
        </span>
      ))}
      <span className="prediction-hint">Space</span>
    </span>
  );
}

function loadSetting(key: string, fallback: boolean): boolean {
  try {
    const value = localStorage.getItem(`jis-korean:${key}`);
    return value === null ? fallback : value === "true";
  } catch {
    return fallback;
  }
}

function saveSetting(key: string, value: boolean) {
  try {
    localStorage.setItem(`jis-korean:${key}`, String(value));
  } catch {
    // Not remembered; fine.
  }
}
