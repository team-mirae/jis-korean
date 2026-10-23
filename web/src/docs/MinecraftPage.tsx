import { Code, PageHead, Source, Table } from "./Docs";

export function MinecraftPage() {
  return (
    <>
      <PageHead title="Minecraft" folder="minecraft/">
        A Fabric mod (client only) that types Korean in Minecraft's text fields with the JIS Korean layout, and decides
        word by word whether you meant Korean or English.
      </PageHead>

      <h2>What it is</h2>
      <ul>
        <li>
          For Minecraft 26.3 (and not 26.4), Fabric Loader 0.19.3+, Fabric API and Java 25. 26.3 moved input from GLFW to
          SDL, and the mod reads SDL scancodes, so no earlier version can load it.
        </li>
        <li>
          Works in single-line text fields: chat, commands, anvils, command blocks and search boxes. Books and signs use
          other widgets and aren't supported yet.
        </li>
        <li>
          <strong>Always on, whatever the input source.</strong> In those fields the mod types with its own engine from the
          raw key presses, and keeps the OS's text input off, so no input method (JIS Korean included) composes the same
          keys a second time. Switch input sources as you like.
        </li>
        <li>
          <strong>The mode follows JIS Korean's.</strong> Each time a screen opens, the mod reads the installed input
          method's mode (on macOS <code>CurrentModeID</code>, on Windows the <code>Mode</code> setting) and types Mode A or
          Mode B to match. Without JIS Korean installed, it's Mode A.
        </li>
      </ul>

      <h2>How it was made to work in Minecraft</h2>

      <h3>The engine runs on Chicory</h3>
      <p>
        The game is Java and the engine is Rust. Instead of a native library per OS, the build compiles the engine to
        WebAssembly (<code>wasm32-unknown-unknown</code>) and bundles it in the mod jar, and the mod runs it on{" "}
        <a href="https://chicory.dev" target="_blank" rel="noreferrer">
          Chicory
        </a>
        , a WebAssembly runtime written in pure Java. One engine binary for every OS, no native code, and the same C ABI
        as the macOS input method and the web playground (<Source path="minecraft/src/main/java/me/sjun/jiskorean/engine/WasmEngine.java">WasmEngine.java</Source>).
      </p>

      <h3>Catching keys before the text field</h3>
      <p>
        Minecraft has no input method framework, so the mod sits between the keyboard and the focused field using Fabric
        API's screen events (<Source path="minecraft/src/client/java/me/sjun/jiskorean/client/JisKoreanClient.java">JisKoreanClient.java</Source>):
      </p>
      <ul>
        <li>
          <code>ScreenEvents.AFTER_INIT</code> registers, on every screen, <code>allowKeyPress</code> and{" "}
          <code>allowCharType</code> listeners and a render hook.
        </li>
        <li>
          On a key press, it finds the focused <code>EditBox</code>, looking through nested containers, and only acts if the
          field takes input. The key goes to the staging area; whatever the staging area commits is inserted with{" "}
          <code>insertText</code>, and the key is cancelled when the staging area used it.
        </li>
        <li>
          Minecraft also sends a separate character event for each key. When the staging area took the key press, the
          matching character event is dropped, so no Latin letter lands next to the Hangul.
        </li>
        <li>Moving focus to another field, or closing the screen, clears the staging area.</li>
      </ul>

      <h3>Physical keys from SDL scancodes</h3>
      <p>
        <code>KeyEvent.key()</code> in 26.3 is an SDL scancode: a USB HID position, independent of the OS keyboard layout.{" "}
        <Source path="minecraft/src/main/java/me/sjun/jiskorean/engine/Keys.java">Keys.java</Source> maps it to the engine's{" "}
        <code>KeyboardEvent.code</code> names, including ¥ (international 3) and ろ (international 1). Shift is read from
        the held-key state too, since it decides between ㅂ and ㅃ. On Macs where Karabiner turns ろ into Option+¥, the mod
        reads Option+¥ back as ろ.
      </p>

      <h3>A staging area, read two ways</h3>
      <p>
        Every key goes into a staging area, drawn in a small window above the caret. The engine gives two readings of the
        same keys: the Korean it composes (<code>gksrmf</code> → 한글) and the English they type on a plain layout of the same
        keyboard, JIS in Mode A and US ANSI in Mode B (<code>gksrmf</code>). Space, Enter or Tab commit the likelier one: Space adds a space, Enter then sends the line,
        and Tab only commits. Minecraft's own suggestion line isn't used, so command suggestions keep working (
        <Source path="minecraft/src/main/java/me/sjun/jiskorean/Staging.java">Staging.java</Source>, which has no Minecraft
        types, so the whole flow is unit-tested).
      </p>
      <Table
        head={["Key", "Effect"]}
        rows={[
          ["Space / Enter / Tab", "Commit the ranked reading"],
          ["F6", "Force Korean (the engine's F6, which commits)"],
          ["F7", "Force English, in place of the engine's hanja"],
          ["F8 / Shift+F8, F9, F10", "As in JIS Korean"],
          ["Backspace", "Edits the staging area by jamo, or deletes in the field when it's empty"],
          ["Escape", "Drops the staging area, then closes the screen"],
        ]}
      />
      <p>F6 and F7 choices are remembered for the session: force a word to English once and it stays English.</p>

      <h3>Ranking Korean against English</h3>
      <p>
        <Source path="minecraft/src/main/java/me/sjun/jiskorean/rank/Ranker.java">Ranker.java</Source> compares two
        key-trigram models on the keys pressed: one trained on English spelling, one on the keys JIS Korean users press
        for Korean words. Rules on top:
      </p>
      <ul>
        <li>
          Shift on a key Korean never shifts (anything but Q W E R T O P) means English, unless the Korean reading is a
          known word: Shift held over from ㅆ (<code>dlTSms</code> is 있는).
        </li>
        <li>
          No letter or digit key at all (Shift+digits, <code>?</code>, <code>!</code>, <code>^^</code>) means Korean: what
          the keys' legends show.
        </li>
        <li>Exact matches in the common-word lists lean their way.</li>
        <li>Leftover bare jamo count against Korean, except chat words like ㅋㅋ.</li>
        <li>
          A <code>/command</code> name is English, unless it's one of the Mirae server's Korean aliases. Its arguments are
          ranked as usual.
        </li>
        <li>The previous word's language tips close calls.</li>
      </ul>
      <Table
        head={["Held-out test", "Accuracy"]}
        rows={[
          ["English words", "99.5%"],
          ["Korean words (JIS Korean typing)", "99.2%"],
          ["Real Korean text, word by word (not trained on)", "about 96%"],
        ]}
      />

      <h3>Drawing and settings</h3>
      <p>
        The staging window is drawn in the screen's render pass, above the focused field's caret (
        <Source path="minecraft/src/client/java/me/sjun/jiskorean/client/CandidateWindow.java">CandidateWindow.java</Source>
        ). The mod has no settings of its own: it's always on, in the mode JIS Korean is in.
      </p>

      <h2>Build and test</h2>
      <Code lang="sh">{`
cd minecraft
./gradlew build               # the mod jar, with the engine, model and Chicory inside
./gradlew test                # staging, ranking and engine tests, no game
./gradlew runClientGameTest   # boots the real client, opens chat and types through the mod
`}</Code>
    </>
  );
}
