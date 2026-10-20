import { Code, PageHead, Source, Table } from "./Docs";

export function EnginePage() {
  return (
    <>
      <PageHead title="Engine" folder="engine/">
        The core of JIS Korean, in Rust. Every platform runs this same engine: it decides what each key types, composes
        Hangul, commits text, and ranks predictions. The platforms only move key events in and text out.
      </PageHead>

      <h2>What it is</h2>
      <ul>
        <li>
          <strong>Event in, actions out.</strong> A key press goes in as a physical key (its <code>KeyboardEvent.code</code>{" "}
          name) and modifiers; out come the text to commit, the unconfirmed text to show (the preedit), and whether the key
          was used. No callbacks, no global state, no I/O, no OS APIs. A shell keeps one engine per text field.
        </li>
        <li>
          <strong>Deterministic and crash-free.</strong> The same keys always give the same text, and no input makes it
          panic. On Windows it runs inside every app's process, so the C ABI also catches any unwinding.
        </li>
        <li>
          <strong>Two builds.</strong> The full build has the predictive layer (the <code>predict</code> feature, the
          default). The lite build (<code>--no-default-features</code>) is the deterministic engine alone.
        </li>
        <li>
          <strong>Targets.</strong> A static library for macOS (arm64 and x86_64), and WebAssembly for the web playground and
          the Minecraft mod.
        </li>
        <li>
          <strong>The spec decides.</strong> All behavior is defined in <Source path="docs/spec.md" />. Every example in it is
          a test.
        </li>
      </ul>

      <h2>Core logic</h2>
      <p>Each key press goes through the same steps:</p>
      <ol>
        <li>
          <strong>Key map</strong> (<Source path="engine/src/layout.rs" />): the mode's table says what the key types, as a
          Hangul jamo or as a character. The 26 letter keys always type standard 2-set in every mode. Mode A is the
          original layout for JIS keyboards; Mode B carries its number row and brackets onto US ANSI and leaves every other
          key as US ANSI has it; Mode C does the same on UK ISO, with · and — on the extra key left of Z (spec §3.6).
        </li>
        <li>
          <strong>Segment</strong> (<Source path="engine/src/engine.rs" />): the key joins the unconfirmed segment as a
          token, which keeps both what it typed and what the same key types on the plain keyboard underneath (its English
          reading).
        </li>
        <li>
          <strong>Composition</strong> (<Source path="engine/src/hangul.rs" />): the segment's jamo are composed into
          syllables by the 2-set automaton, with JIS Korean's additions: the implied ㅇ (ㅏ alone types 아), Shift+vowel
          starting a syllable on ㅇ, and the double press.
        </li>
        <li>
          <strong>Commit</strong>: Space, Enter, shortcuts, focus loss and switching input source commit the segment
          (spec §5). F6 commits too, after dropping a plain implied ㅇ or undoing F9/F10.
        </li>
        <li>
          <strong>Function keys</strong>: F7 opens hanja candidates for the longest dictionary word at the end of the
          segment, F8 escapes to standard 2-set on the plain keys, F9 and F10 convert the word to full- and half-width
          letters, cycling the case. Shift+Z is F10.
        </li>
        <li>
          <strong>Predictions</strong> (full build, <Source path="engine/src/predict.rs" />): the same keys are read several
          ways (Korean, English, plain 2-set, finals kept, English + particle, hanja) and ranked by key-trigram models,
          word lists and the user's weights. At most {4} readings are offered. Learning from the user's choices is opt-in
          and stays on the device.
        </li>
      </ol>

      <h2>Rust API</h2>
      <Code lang="rust">{`
use jis_korean::{Engine, Key, Mode, Mods};

let mut engine = Engine::new();            // Mode A, no dictionary, no predictor
engine.set_mode(Mode::B);                  // US ANSI
for code in ["KeyG", "KeyK", "KeyS"] {
    let out = engine.key(Key::from_code(code), Mods::NONE);
    // out.commit: text to insert; out.preedit: text to show as marked;
    // out.consumed: false = pass the key on to the app
}
assert_eq!(engine.preedit(), "한");
let out = engine.flush();                  // focus loss: commits "한"
`}</Code>
      <Table
        code
        head={["Item", "What it does"]}
        rows={[
          ["Engine::key(key, mods) -> Output", "One key press. Output: commit, preedit, consumed, the hanja candidates, and the English reading."],
          ["flush() / clear()", "Commit everything unconfirmed / discard it."],
          ["activate()", "The input method was switched back to: leaves the F8 escape."],
          ["set_mode(Mode)", "Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO). Mode::ALL lists them, with id() and name()."],
          ["set_dedicated_symbols(bool)", "Mode C's Dedicated symbol keys, on by default. Off, [ ] and the ISO extra key type their UK characters."],
          ["set_enter_commits(bool)", "Plain Enter commits without a new line, as Shift+Enter does."],
          ["set_dictionary(Arc<Dictionary>)", "The hanja dictionary for F7, shared between engines."],
          ["set_predictor(Option<Arc<Predictor>>)", "Turns predictions on (full build). predictions() ranks; commit_prediction(i) commits one."],
          ["preedit() / english()", "The unconfirmed text, and the same keys read on the plain keyboard."],
          ["output_in(mode, layout, key, shift)", "What a key types in a mode, for drawing legends."],
          ["resolve::*", "Resolving keys and text (below)."],
        ]}
      />

      <h2>C ABI</h2>
      <p>
        The shells use the C ABI in <Source path="engine/include/jis_korean.h" />. Conventions:
      </p>
      <ul>
        <li>
          An engine call stores its result in the engine handle; read it with the matching <code>_ptr</code> and{" "}
          <code>_len</code> functions until the next call. Strings are UTF-8 and not NUL-terminated.
        </li>
        <li>
          Keys are passed as stable ids (<code>JK_KEY_*</code>), one per <code>KeyboardEvent.code</code>. New keys are only
          appended, before <code>JK_KEY_MODIFIER</code>; a test fails if the header and the engine disagree.
        </li>
        <li>One handle per text field, used from one thread. Dictionaries and predictors are shared, reference-counted.</li>
        <li>Wasm hosts pass data in through <code>jk_alloc</code> / <code>jk_dealloc</code>.</li>
      </ul>
      <Table
        head={["Group", "Functions"]}
        rows={[
          ["Lifecycle", <code>jk_new, jk_free</code>],
          ["Keys", <code>jk_key, jk_flush, jk_clear, jk_activate, jk_set_enter_commits, jk_is_escaped</code>],
          ["Results", <code>jk_commit_*, jk_preedit_*, jk_english_*, jk_candidate_*</code>],
          ["Modes", <code>jk_set_mode, jk_mode_count, jk_mode_name_*, jk_legend, jk_legend_in</code>],
          ["Keys by name", <code>jk_key_count, jk_key_code_ptr, jk_key_code_len</code>],
          ["Resolving", <code>jk_resolve_key_input, jk_resolve_key_input_after, jk_resolve_keys_for_output, jk_resolved_*</code>],
          ["Hanja", <code>jk_dict_new, jk_dict_free, jk_set_dictionary</code>],
          ["Predictions", <code>jk_predictor_new, jk_set_predictor, jk_predict, jk_prediction_*, jk_commit_prediction, jk_set_prediction_weight, jk_set_ranker</code>],
          ["Learning", <code>jk_set_learning, jk_set_recording, jk_learned_weight, jk_learner_*, jk_learn_example, jk_predictor_with_words</code>],
          ["Key counts", <code>jk_plan, jk_plan_jis, jk_plan_conventional, jk_plan_chars, jk_plan_off_keyboard, jk_plan_dropped</code>],
        ]}
      />

      <h2>Resolving keys and text</h2>
      <p>
        Two questions, answered on a copy of a live engine so the answers match it exactly: its mode, and with predictions
        on, its predictor, weights and what it learned (<Source path="engine/src/resolve.rs" />, spec §8.3). A key press is
        written as its <code>KeyboardEvent.code</code>, with <code>S-</code> for Shift: <code>KeyG</code>,{" "}
        <code>S-Digit2</code>. One press is one key, with or without Shift.
      </p>
      <Table
        code
        head={["Function", "Returns", "Example"]}
        rows={[
          ["key_input(engine, press)", "The character the key types on its own, or none (F-keys, Space, Shift+Z).", <code>KeyG → ㅎ</code>],
          ["key_input_after(engine, context, press)", "What the field shows after the unconfirmed context and the key, including anything it commits.", <code>하 + KeyS → 한</code>],
          [
            "keys_for_output(engine, text, min, max)",
            "Every combination of presses that types the text, min to max presses, fewest presses then fewest Shifts first, at most 1,000. Never an error: a nonsensical range gives an empty list, and max is cut to 16.",
            <code>ㅃ → [Digit1], [S-KeyQ]</code>,
          ],
          ["type_keys(engine, presses)", "What the field shows after typing the presses from an empty field.", <code>[KeyG, KeyK, KeyS] → 한</code>],
        ]}
      />
      <p>
        Keys apply as a shell applies them: with predictions on, the marked text shows the top prediction unless it's the
        Korean reading, and Space and Enter commit what's shown. The search tries only keys whose characters are part of
        the text, skips presses that change nothing, and follows routes heading toward the text: its jamo, its letters
        before F9/F10, a hanja's reading before F7. Every combination is checked by typing it.
      </p>

      <h2>Performance</h2>
      <h3>Time</h3>
      <Table
        head={["Operation", "Cost", "Why"]}
        rows={[
          ["A key press (lite)", <>O(<i>n</i>), <i>n</i> = keys staged</>, "The segment is recomposed for the preedit; a segment is one word, usually under 20 keys."],
          ["A key press with predictions", <>O(<i>n</i> · <i>k</i>), <i>k</i> ≤ 8</>, "Each reading is composed (a handful), plus the English + particle splits over the last 8 keys; model and word-list lookups are hash lookups, O(1) each."],
          ["F7 (hanja)", <>O(<i>w</i>), <i>w</i> = syllables at the end</>, "One hash lookup per suffix length, longest first."],
          ["Loading the dictionary", "O(size)", "167,783 readings from a 4.3 MB TSV, parsed once and shared."],
          ["key_input", "O(1)", "One table lookup."],
          ["key_input_after, keys_for_output", "Bounded", "A search over key presses, capped at 50,000 engine steps and 16 presses: it always ends, and returns what it found."],
        ]}
      />
      <h3>Measured</h3>
      <p>
        Native release build on an Apple M6, typing <Source path="web/public/long-sample-text.txt" /> word by word (4,313 key
        presses, Korean and English), with predictions ranked after every press as a shell does. Run it yourself with{" "}
        <code>cargo run --release --example bench [lite|full|resolve]</code>.
      </p>
      <Table
        head={["Measurement", "Lite", "With predictions"]}
        rows={[
          ["Key press, mean", "0.2 µs", "3.6 µs"],
          ["Key press, median", "0.2 µs", "2.7 µs"],
          ["Key press, 99th percentile", "0.7 µs", "16.2 µs"],
          ["Key press, slowest", "1.3 µs", "54 µs"],
          ["Load the hanja dictionary", "31–46 ms", "31–46 ms"],
          ["Load the predictor", "–", "8–11 ms"],
        ]}
      />
      <Table
        head={["Resolving (with predictions)", "Result", "Time"]}
        rows={[
          [<code>keys_for_output("한", 3, 3)</code>, "8 combinations", "0.3 ms"],
          [<code>keys_for_output("hello", 5, 5)</code>, "1 combination", "3.6 ms"],
          [<code>key_input_after("대한민국의", KeyS)</code>, "대한민국읜", "0.2 ms"],
          [<code>keys_for_output("한글", 1, 8)</code>, "844 combinations", "91 ms"],
          [<code>keys_for_output("學校", 6, 6)</code>, "8 combinations", "91 ms (builds a reverse hanja index)"],
        ]}
      />
      <h3>Memory</h3>
      <Table
        head={["Part", "Size", "Notes"]}
        rows={[
          ["Engine per text field", "Small", "The segment (one word of tokens), settings and learned weights. Engines share the dictionary and predictor."],
          ["Hanja dictionary", "4.3 MB on disk; most of a 74 MB process", "Parsed into a hash map of 167,783 readings. Peak resident memory of the whole benchmark process, lite: 74 MB."],
          ["Predictor", "1.3 MB on disk; about 8 MB in memory", "The key-trigram model (39 KB) and three word lists. Peak resident memory with it: 82 MB."],
          ["Wasm binary", "445 KB (146 KB gzipped)", "The full engine with predictions and resolving, without data files."],
        ]}
      />

      <h2>Tests</h2>
      <p>
        <code>cargo test</code> in <code>engine/</code>; <code>cargo test --no-default-features</code> for the lite build.
        Golden tables cover every automaton transition, Backspace, each F-key and every spec example; a robot types real
        text key by key and checks the screen after every character; <code>tests/modes.rs</code> checks every mode against
        the mode contract (<Source path="docs/modes.md" />).
      </p>
    </>
  );
}
