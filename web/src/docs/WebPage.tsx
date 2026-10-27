import { Code, PageHead, Source, Table } from "./Docs";

export function WebPage() {
  return (
    <>
      <PageHead title="Web" folder="web/">
        This site: a React playground that runs the real engine in your browser, so you can try JIS Korean before
        installing it, and these docs.
      </PageHead>

      <h2>What it is</h2>
      <ul>
        <li>
          <strong>The same engine,</strong> compiled to WebAssembly and loaded through its C ABI by{" "}
          <Source path="web/src/engine.ts">engine.ts</Source>. Nothing is sent anywhere: typing, predictions and learning stay
          in the page.
        </li>
        <li>
          <strong>Physical keys.</strong> The page reads <code>KeyboardEvent.code</code>, so it types JIS Korean whatever
          your computer's layout is, as long as the input source is English. On keyboards without ¥ or ろ, use the on-screen
          keys.
        </li>
        <li>
          <strong>Predictions on by default,</strong> with the weights, rankers and learning exposed for tuning, and a
          key-press comparison against conventional 2-set.
        </li>
        <li>
          React and Vite, with no other dependencies. Deployed at jis-korean.sjun.me from <code>main</code> by GitHub
          Actions, after the engine tests pass.
        </li>
      </ul>
      <Code lang="sh">{`
cd web
npm run dev      # builds the engine to wasm, then starts Vite
npm run build    # the production build in dist/
`}</Code>

      <h2>Engine API in the browser</h2>
      <p>
        <code>loadEngine()</code> instantiates the wasm and returns an <code>Engine</code>, one per text field:
      </p>
      <Code lang="ts">{`
import { loadEngine } from "./engine";

const engine = await loadEngine();
await engine.loadDictionary("/hanja.tsv");      // for F7
engine.key("KeyG"); engine.key("KeyK");
const out = engine.key("KeyS");                  // { commit: "", preedit: "한", consumed: true, … }

engine.setMode(1);                               // Mode B, US ANSI
engine.resolveKeyCodeInput("KeyS", "하");        // "한"
engine.resolveKeyCodeFromOutput("ㅃ", 1, 1);     // [["Digit1"], ["S-KeyQ"]]
`}</Code>
      <Table
        code
        head={["Method", "What it does"]}
        rows={[
          ["key(code, mods?)", "One key press: { commit, preedit, consumed, candidates }."],
          ["flush() / activate()", "Commit everything / note the input method was switched back to."],
          ["setEnterCommits(on)", "Plain Enter commits without a new line."],
          ["modes() / setMode(id)", "The modes and their names; switch between Mode A (0), Mode B (1) and Mode C (2). setDedicatedSymbols(on) is Mode C's Dedicated symbol keys."],
          ["legend(code, shift, escaped)", "What a key types in the current mode, for drawing keys."],
          ["resolveKeyCodeInput(press, context?)", "What a key types alone, or the field after a context and the key. Null if none."],
          ["resolveKeyCodeFromOutput(text, min?, max?)", "Every combination of presses that types the text. Empty for a nonsensical range."],
          ["loadDictionary(url)", "The hanja dictionary, for F7."],
          ["loadPredictor(base) / setPredictions(on)", "Load the predictive layer, and turn it on or off."],
          ["predictions() / commitPrediction(i)", "The ranked readings, and committing one. predictionsBy / commitPredictionBy do the same with another ranker, for comparing them."],
          ["setWeight / clearWeight / setRanker", "Tune the ranking (spec §10.5)."],
          ["setLearning / exportLearner / importLearner / resetLearner / learnedWeight", "Learning from choices, kept by the page."],
          ["setUserWords / clearUserWords / learnExample", "The training file's words and examples."],
          ["plan(text)", "Key presses for a text on JIS Korean and on conventional 2-set."],
        ]}
      />
    </>
  );
}
