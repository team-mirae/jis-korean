import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Candidates, Counts, Engine, Mods, Output, Prediction, PredictionKind, RankerName, WeightName } from "./engine";
import { WEIGHTS } from "./engine";
import type { TrainingFile, TrainingSummary } from "./training";
import { check, learnExamples, wordLists } from "./training";
import { ghost } from "./ghost";
import { Keyboard } from "./Keyboard";
import benchmarks from "./stats.json";

const REPO = "https://github.com/team-mirae/jis-korean";

/** Short runs on the number row and home row that type whole words (docs/easy-words.md). */
const EASY_WORDS: [string, string][] = [
  ["98", "의외"],
  ["09", "와의"],
  ["a7", "뭐"],
  ["3h", "또"],
  ["h1k", "오빠"],
  ["98fh", "의외로"],
  ["k4k7", "아까워"],
  ["l3k6", "이따위"],
];

/** Practice texts in public/: the gray guide in the typing area, and what's counted when it's empty. */
const SAMPLES = [
  { id: "short", file: "sample.txt", label: "Short" },
  { id: "long", file: "long-sample-text.txt", label: "AI since ChatGPT (long)" },
];

/** The keyboard floats over the page once the typing area is taller than this many lines. */
const FLOAT_AFTER_LINES = 15;

export function App({ engine }: { engine: Engine }) {
  const [doc, setDoc] = useState("");
  const [preedit, setPreedit] = useState("");
  const [escaped, setEscaped] = useState(false);
  const [shift, setShift] = useState(false);
  const [enterCommits, setEnterCommits] = useState(false);
  // Mode B (spec §3.6): JIS Korean for US ANSI keyboards. Remembered per browser.
  // The engine's mode is set before the first render, so the key legends match it.
  const [modeB, setModeB] = useState(() => {
    const b = loadSetting("modeB", false);
    engine.setMode(b ? 1 : 0);
    return b;
  });
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
  // Side by side: the travel ranker's list next to the rules' (spec §10.7).
  const [ranking, setRanking] = useState<Ranking>(() => loadRanking());
  const [travelReadings, setTravelReadings] = useState<Prediction[]>([]);
  const [routeReadings, setRouteReadings] = useState<Prediction[]>([]);
  const [weights, setWeights] = useState<Record<WeightName, number>>(() => loadWeights());
  // Learning from the user's choices (spec §10.11): off by default, kept only in this browser and
  // forgotten 24 hours after it started, session or not.
  const [learningOn, setLearningOn] = useState(() => loadSetting("learning", false));
  const [learned, setLearned] = useState<Record<WeightName, number>>(() => unlearned());
  // A training file (docs/training-contract.md): what it applied, or why it couldn't.
  const [training, setTraining] = useState<{ summary?: TrainingSummary; problems?: string[] } | null>(null);
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
      .then(() => {
        engine.setPredictions(predictionsOn);
        engine.setRanker(ranking === "side-by-side" ? "rules" : ranking);
        // Only weights the user set by hand are pinned; the rest are left to learning.
        for (const [name, value] of Object.entries(storedWeights())) engine.setWeight(name as WeightName, value);
        const saved = loadLearned();
        if (saved) engine.importLearner(saved.state);
        const words = loadTrainingWords();
        if (words) engine.setUserWords(words.korean, words.english);
        engine.setLearning(learningOn);
        setLearned(learnedWeights(engine));
      })
      .catch((error) => console.error("predictor:", error));
    // Only once: later changes go through togglePredictions.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [engine]);
  useEffect(() => {
    // The 24-hour limit holds within a session too: check once a minute.
    const timer = setInterval(() => {
      if (expireLearned()) {
        engine.resetLearner();
        setLearned(learnedWeights(engine));
      }
    }, 60_000);
    return () => clearInterval(timer);
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
    showReadings(Boolean(out.preedit && !out.candidates));
    setEscaped(engine.escaped);
    // Past 24 hours, what was learned goes, counts in the engine included, before anything new
    // is stored (else old counts would carry into a fresh window).
    if (expireLearned()) {
      engine.resetLearner();
      setLearned(learnedWeights(engine));
    } else if (engine.learnerChanged()) {
      saveLearned(engine.exportLearner());
      setLearned(learnedWeights(engine));
    }
  }

  function type(code: string, mods: Mods): Output {
    const out = engine.key(code, mods);
    apply(out, code, mods);
    return out;
  }

  /** Switches between Mode A and Mode B, committing what's staged first. */
  function changeMode(b: boolean) {
    if (b === modeB) return;
    apply(engine.flush(), "", {});
    engine.setMode(b ? 1 : 0);
    setModeB(b);
    saveSetting("modeB", b);
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

  /** The predictions (and, side by side, the travel ranker's) of what's staged, or none. */
  function showReadings(show: boolean, mode: Ranking = ranking) {
    setReadings(show ? engine.predictions() : []);
    setTravelReadings(show && mode === "side-by-side" ? engine.predictionsBy("travel") : []);
    setRouteReadings(show && mode === "side-by-side" ? engine.predictionsBy("route") : []);
  }

  /** Commits reading `index` of the predictions (of `ranker`'s list, side by side), then `suffix`. */
  function pick(index: number, suffix = "", ranker?: RankerName) {
    const out = ranker ? engine.commitPredictionBy(ranker, index) : engine.commitPrediction(index);
    apply({ ...out, commit: out.commit + suffix }, "", {});
  }

  function changeRanking(mode: Ranking) {
    setRanking(mode);
    try {
      localStorage.setItem("jis-korean:ranking", mode);
    } catch {
      // Not remembered; fine.
    }
    engine.setRanker(mode === "side-by-side" ? "rules" : mode);
    showReadings(predictionsOn && Boolean(preedit), mode);
  }

  function togglePredictions(on: boolean) {
    setPredictionsOn(on);
    saveSetting("predictions", on);
    engine.setPredictions(on);
    showReadings(on && Boolean(preedit));
    surface.current?.focus();
  }

  /** A weight set by hand: it wins over the learned one. */
  function setWeight(name: WeightName, value: number) {
    setWeights({ ...weights, [name]: value });
    engine.setWeight(name, value);
    try {
      localStorage.setItem("jis-korean:weights", JSON.stringify({ ...storedWeights(), [name]: value }));
    } catch {
      // Not remembered; fine.
    }
    showReadings(predictionsOn && Boolean(preedit));
  }

  /** Back to the defaults: every weight handed back to learning. */
  function resetWeights() {
    for (const name of WEIGHTS) engine.clearWeight(name);
    setWeights(loadWeights(true));
    try {
      localStorage.removeItem("jis-korean:weights");
    } catch {
      // Nothing stored; fine.
    }
    showReadings(predictionsOn && Boolean(preedit));
  }

  function toggleLearning(on: boolean) {
    setLearningOn(on);
    saveSetting("learning", on);
    engine.setLearning(on);
    showReadings(predictionsOn && Boolean(preedit));
  }

  /** Applies a training file: its weights as hand-set sliders, its words (kept in this browser),
   * and its examples to the learner (which only counts while learning is on, so it's turned on). */
  function applyTraining(file: unknown) {
    const problems = check(file);
    if (problems.length) {
      setTraining({ problems });
      return;
    }
    const f = file as TrainingFile;
    let next = { ...weights };
    for (const [name, value] of Object.entries(f.weights ?? {}) as [WeightName, number][]) {
      next = { ...next, [name]: value };
      engine.setWeight(name, value);
    }
    setWeights(next);
    try {
      localStorage.setItem("jis-korean:weights", JSON.stringify({ ...storedWeights(), ...(f.weights ?? {}) }));
    } catch {
      // Not remembered; fine.
    }
    const lists = wordLists(f);
    const count = (f.vocabulary?.korean?.length ?? 0) + (f.vocabulary?.english?.length ?? 0);
    if (count) {
      engine.setUserWords(lists.korean, lists.english);
      saveTrainingWords(lists);
    }
    if (f.examples?.length && !learningOn) toggleLearning(true);
    if (expireLearned()) engine.resetLearner();
    const examples = learnExamples(engine, f);
    if (engine.learnerChanged()) saveLearned(engine.exportLearner());
    setLearned(learnedWeights(engine));
    setTraining({
      summary: {
        source: f.source?.tool ?? "training file",
        weights: Object.keys(f.weights ?? {}).length,
        words: count,
        examples,
      },
    });
    showReadings(predictionsOn && Boolean(preedit));
  }

  function forgetTrainingWords() {
    engine.clearUserWords();
    try {
      localStorage.removeItem(TRAINING_WORDS_KEY);
    } catch {
      // Nothing stored; fine.
    }
    setTraining(null);
    showReadings(predictionsOn && Boolean(preedit));
  }

  function forgetLearned() {
    engine.resetLearner();
    try {
      localStorage.removeItem(LEARNED_KEY);
    } catch {
      // Nothing stored; fine.
    }
    setLearned(learnedWeights(engine));
    showReadings(predictionsOn && Boolean(preedit));
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
        </div>
        <p className="lede">
          Korean 2-set (두벌식) for JIS keyboards: double consonants and compound vowels on the number row, Korean
          punctuation on the symbol keys, and Japanese-IME-style F6–F10. Try it here before installing; on a US
          keyboard, try Mode B.
        </p>
      </header>

      <section className="editor" aria-label="Try it">
        <div className="status">
          <span className="samples" role="group" aria-label="Mode">
            {[
              { b: false, label: "Mode A · JIS", title: "JIS Korean Original Layout, for JIS keyboards" },
              { b: true, label: "Mode B · US ANSI", title: "JIS Korean for US ANSI keyboards" },
            ].map(({ b, label, title }) => (
              <button
                key={label}
                type="button"
                title={title}
                aria-pressed={modeB === b}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => changeMode(b)}
              >
                {label}
              </button>
            ))}
          </span>
          <span className={"chip" + (escaped ? " escaped" : "")}>
            {escaped ? "Standard 2-set · Shift+F8 returns" : modeB ? "JIS Korean for US ANSI" : "JIS Korean"}
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
          {!candidates && readings.length > 0 && (
            <PredictionList
              readings={readings}
              label={ranking === "side-by-side" ? "Rules" : undefined}
              onPick={(i) => pick(i)}
            />
          )}
          {!candidates && travelReadings.length > 0 && (
            <PredictionList readings={travelReadings} label="Travel" onPick={(i) => pick(i, "", "travel")} />
          )}
          {!candidates && routeReadings.length > 0 && (
            <PredictionList readings={routeReadings} label="Route" onPick={(i) => pick(i, "", "route")} />
          )}
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
        {predictionsOn && (
          <Tuning
            weights={weights}
            onChange={setWeight}
            onReset={resetWeights}
            ranking={ranking}
            onRanking={changeRanking}
            learningOn={learningOn}
            onLearning={toggleLearning}
            learned={learned}
            onForget={forgetLearned}
            training={training}
            onTraining={applyTraining}
            onForgetTraining={forgetTrainingWords}
          />
        )}
      </section>

      <Keyboard
        engine={engine}
        ansi={modeB}
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
            <dt>Easy words</dt>
            <dd>
              {EASY_WORDS.map(([keys, word], i) => (
                <span key={keys}>
                  {i > 0 && " · "}
                  <span className="easy-word">
                    <code>{keys}</code> {word}
                  </span>
                </span>
              ))}
              . <a href={`${REPO}/blob/main/docs/easy-words.md`}>More</a>
            </dd>
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
            <dt>F9 / F10, Shift+Z</dt>
            <dd>The word as full- or half-width letters. Press again to cycle case. Shift+Z does what F10 does, for keyboards and browsers where F10 is taken.</dd>
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
  bare: "ㅏ",
  mixed: "En+한",
  "implied-l": "ㄹ",
};

/** The predictive layer's ranked readings of the staged text, top one first. */
function PredictionList({ readings, label, onPick }: { readings: Prediction[]; label?: string; onPick(index: number): void }) {
  return (
    <span className="predictions" role="listbox" aria-label={label ? `Predictions (${label})` : "Predictions"}>
      {label && <span className="ranker-label">{label}</span>}
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
      {(!label || label === "Rules") && <span className="prediction-hint">Space</span>}
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

const WEIGHT_LABELS: Record<WeightName, string> = {
  korean: "Korean",
  english: "English",
  implied: "Implied ㅇ",
  mixed: "English + particle",
  "two-set": "Plain 2-set",
  "keep-finals": "Finals kept",
  hanja: "Hanja",
  "implied-l": "Implied ㄹ after ㄴ",
};

/** Scenarios a slider can turn off (at 0); the rest go down to 0.5. */
const CAN_TURN_OFF: WeightName[] = ["implied-l"];

/** Which ranker orders the implied-ㅇ alternatives, or both lists side by side. */
type Ranking = "rules" | "travel" | "route" | "side-by-side";

const RANKINGS: { value: Ranking; label: string }[] = [
  { value: "rules", label: "Rules" },
  { value: "travel", label: "Travel (experimental)" },
  { value: "route", label: "Route preference (experimental)" },
  { value: "side-by-side", label: "Side by side" },
];

function loadRanking(): Ranking {
  try {
    const value = localStorage.getItem("jis-korean:ranking");
    return RANKINGS.some((r) => r.value === value) ? (value as Ranking) : "rules";
  } catch {
    return "rules";
  }
}

function unlearned(): Record<WeightName, number> {
  return Object.fromEntries(WEIGHTS.map((w) => [w, 1])) as Record<WeightName, number>;
}

/** The weights the user set by hand (only those). */
function storedWeights(): Partial<Record<WeightName, number>> {
  try {
    const stored = JSON.parse(localStorage.getItem("jis-korean:weights") ?? "{}");
    return Object.fromEntries(Object.entries(stored).filter(([k, v]) => (WEIGHTS as readonly string[]).includes(k) && typeof v === "number"));
  } catch {
    return {};
  }
}

function loadWeights(reset = false): Record<WeightName, number> {
  return reset ? unlearned() : { ...unlearned(), ...storedWeights() };
}

function learnedWeights(engine: Engine): Record<WeightName, number> {
  return Object.fromEntries(WEIGHTS.map((w) => [w, engine.learnedWeight(w)])) as Record<WeightName, number>;
}

// The words from a training file, kept in this browser until forgotten.
const TRAINING_WORDS_KEY = "jis-korean:training-words";

function loadTrainingWords(): { korean: string; english: string } | null {
  try {
    const saved = JSON.parse(localStorage.getItem(TRAINING_WORDS_KEY) ?? "null");
    return saved && typeof saved.korean === "string" && typeof saved.english === "string" ? saved : null;
  } catch {
    return null;
  }
}

function saveTrainingWords(words: { korean: string; english: string }) {
  try {
    localStorage.setItem(TRAINING_WORDS_KEY, JSON.stringify(words));
  } catch {
    // Not remembered; they apply for this page only.
  }
}

// What was learned, with when learning started: forgotten 24 hours after that, session or not.
const LEARNED_KEY = "jis-korean:learned";
const LEARNED_FOR_MS = 24 * 60 * 60 * 1000;

function loadLearned(): { created: number; state: string } | null {
  try {
    const saved = JSON.parse(localStorage.getItem(LEARNED_KEY) ?? "null");
    if (!saved || typeof saved.created !== "number" || typeof saved.state !== "string") return null;
    if (Date.now() - saved.created >= LEARNED_FOR_MS) {
      localStorage.removeItem(LEARNED_KEY);
      return null;
    }
    return saved;
  } catch {
    return null;
  }
}

/** Stores what was learned; the 24-hour window starts with the first store. Callers expire first. */
function saveLearned(state: string) {
  try {
    const created = loadLearned()?.created ?? Date.now();
    localStorage.setItem(LEARNED_KEY, JSON.stringify({ created, state }));
  } catch {
    // Not remembered; it lives for this page only.
  }
}

/** Forgets what was learned once it's 24 hours old. True if it did. */
function expireLearned(): boolean {
  try {
    const saved = JSON.parse(localStorage.getItem(LEARNED_KEY) ?? "null");
    if (saved && typeof saved.created === "number" && Date.now() - saved.created >= LEARNED_FOR_MS) {
      localStorage.removeItem(LEARNED_KEY);
      return true;
    }
  } catch {
    // Storage unreadable: nothing to expire.
  }
  return false;
}

/** Per-scenario multipliers on the predictor's learned probabilities (spec §10.5). */
function Tuning({
  weights,
  onChange,
  onReset,
  ranking,
  onRanking,
  learningOn,
  onLearning,
  learned,
  onForget,
  training,
  onTraining,
  onForgetTraining,
}: {
  weights: Record<WeightName, number>;
  onChange(name: WeightName, value: number): void;
  onReset(): void;
  ranking: Ranking;
  onRanking(mode: Ranking): void;
  learningOn: boolean;
  onLearning(on: boolean): void;
  learned: Record<WeightName, number>;
  onForget(): void;
  training: { summary?: TrainingSummary; problems?: string[] } | null;
  onTraining(file: unknown): void;
  onForgetTraining(): void;
}) {
  const [over, setOver] = useState(false);
  const read = (file: File | undefined) => {
    if (!file) return;
    file
      .text()
      .then((text) => onTraining(JSON.parse(text)))
      .catch(() => onTraining(null));
  };
  return (
    <details className="tuning">
      <summary>Tune predictions</summary>
      <p className="counts-note">
        Each slider multiplies how likely that reading is, as learned. English 0.8× makes English readings less likely;
        Korean 1.2× makes Korean ones likelier.
      </p>
      <div className="weights">
        {WEIGHTS.map((name) => (
          <label key={name} className="weight">
            <span>{WEIGHT_LABELS[name]}</span>
            <input
              type="range"
              min={CAN_TURN_OFF.includes(name) ? 0 : 0.5}
              max={2}
              step={0.05}
              value={weights[name]}
              onChange={(e) => onChange(name, Number(e.target.value))}
            />
            <output>{weights[name] === 0 ? "off" : `${weights[name].toFixed(2)}×`}</output>
            {learningOn && <small className="learned" title="Learned from your choices">learned {learned[name].toFixed(2)}×</small>}
          </label>
        ))}
      </div>
      <button type="button" className="reset" onClick={onReset}>
        Reset
      </button>
      <fieldset className="ranking">
        <legend>Learning</legend>
        <label>
          <input type="checkbox" checked={learningOn} onChange={(e) => onLearning(e.target.checked)} />
          Learn from my choices (this browser only, forgotten after 24 hours)
        </label>
        <button type="button" className="reset" onClick={onForget}>
          Forget what it learned
        </button>
        <p className="counts-note">
          Counts which kinds of prediction you pick over the top one, nothing else: no text, no keys. It nudges each
          weight within 0.5–2×; a slider you moved wins over it. Hanja never come first.
        </p>
      </fieldset>
      <fieldset className="ranking">
        <legend>Training file</legend>
        <label
          className={"drop" + (over ? " over" : "")}
          onDragOver={(e) => {
            e.preventDefault();
            setOver(true);
          }}
          onDragLeave={() => setOver(false)}
          onDrop={(e) => {
            e.preventDefault();
            setOver(false);
            read(e.dataTransfer.files[0]);
          }}
        >
          <input type="file" accept=".json,application/json" hidden onChange={(e) => read(e.target.files?.[0])} />
          Drop a training file (.json) here, or click to choose one.
        </label>
        {training?.problems && <p className="counts-note">Not applied: {training.problems.join("; ")}.</p>}
        {training?.summary && (
          <p className="counts-note">
            From {training.summary.source}: {training.summary.weights} sliders set, {training.summary.words} words added
            {training.summary.examples.top + training.summary.examples.listed + training.summary.examples.missed.length > 0 &&
              `, examples learned (${training.summary.examples.top} already on top, ${training.summary.examples.listed} listed lower` +
                (training.summary.examples.missed.length ? `, not offered: ${training.summary.examples.missed.slice(0, 8).join(" ")}` : "") +
                ")"}
            .
          </p>
        )}
        <button type="button" className="reset" onClick={onForgetTraining}>
          Forget training words
        </button>
        <p className="counts-note">
          The format is in <a href={`${REPO}/blob/main/docs/training-contract.md`}>docs/training-contract.md</a>: hand it to
          your AI agent with your own data. Read here, in this browser; nothing is uploaded.
        </p>
      </fieldset>
      <fieldset className="ranking">
        <legend>Implied ㅇ ranking</legend>
        {RANKINGS.map((r) => (
          <label key={r.value}>
            <input type="radio" name="ranking" checked={ranking === r.value} onChange={() => onRanking(r.value)} />
            {r.label}
          </label>
        ))}
        <p className="counts-note">
          Travel ranks ㅠㅠ against 유유 and 말이 against 마리 by the keys and finger travel a typist who meant them would
          have skipped. Route preference asks how strongly a typist who meant each reading would prefer the route the keys
          took over typing each jamo. Side by side shows all three lists; click any to commit.
        </p>
      </fieldset>
    </details>
  );
}
