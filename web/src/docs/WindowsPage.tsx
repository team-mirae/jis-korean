import { Code, PageHead, Source, Table } from "./Docs";

export function WindowsPage() {
  return (
    <>
      <PageHead title="Windows" folder="windows/">
        A Text Services Framework (TSF) text input processor, <code>jiskorean.dll</code>, around the engine. It maps scan
        codes to engine keys and shows what the engine says as a TSF composition, the hanja and prediction list, and the
        taskbar's 한글 / ABC indicator.
      </PageHead>

      <h2>What it is</h2>
      <ul>
        <li>
          <strong>A thin shell</strong>, like the macOS input method: scan codes in, the engine's commit and composition
          text out, through the same C ABI (<Source path="engine/include/jis_korean.h" />).
        </li>
        <li>
          <strong>Inside every app's process</strong>, as every TSF input processor is. That's why the engine never panics
          and its C ABI catches any unwinding: a crash would take the app down with it. The DLL links the static C runtime,
          and embeds the hanja dictionary and, in the full build, the prediction model, parsed once per process.
        </li>
        <li>
          <strong>x64 and x86:</strong> 32-bit apps load the x86 DLL. The engine is built with its <code>english</code>{" "}
          feature, for ABC's English layouts.
        </li>
      </ul>

      <h2>Build and install</h2>
      <p>
        Windows 10 or 11, Visual Studio 2022 Build Tools (C++ x64/x86), CMake 3.28+, Ninja, and Rust with the{" "}
        <code>x86_64-pc-windows-msvc</code> and <code>i686-pc-windows-msvc</code> targets.
      </p>
      <Code lang="powershell">{`
cd windows
tools/build.ps1               # x64 + x86, runs the shell tests (-Lite: without predictions)
tools/install.ps1             # admin (UAC): Program Files\\JIS Korean, registers x64 + x86,
                              # adds JIS Korean to the Korean language's keyboards
tools/install.ps1 -Uninstall
tools/install.ps1 -Log        # turn the diagnostic log on and follow it
`}</Code>
      <p>
        Then pick <strong>JIS Korean</strong> with Win+Space, and restart apps that were open: an app keeps the old DLL, or
        none, until it restarts.
      </p>

      <h2>Using it</h2>
      <Table
        head={["Feature", "How it works"]}
        rows={[
          [
            "Modes A, B, C",
            "Right-click the 한글 / ABC indicator → Mode A (JIS), Mode B (US ANSI) or Mode C (UK ISO). One setting for every app: the mode follows the keyboard. In Mode C the menu also has Dedicated Symbol Keys.",
          ],
          [
            "한/영",
            "Switches to English (ABC) and back, as the Microsoft IME does, committing first. One state for the whole session, 한글 after a restart. US ANSI keyboards send it from Right Alt. JIS and ISO keyboards have no 한/영 key, and JIS Korean deliberately doesn't pick one: map the key you want to VK_HANGUL with your keyboard's software or a remapper such as PowerToys. A click on the indicator switches too.",
          ],
          [
            "English Layout (한/영)",
            "What ABC types: by default the keyboard's English (JIS English in Mode A, US English in B, UK English in C), any other English mode, or Windows' own layout.",
          ],
          [
            "Predictions",
            "The unconfirmed text shows the top reading. ↓ / Tab and ↑ move through the list, Space / Enter commit what's shown. Off by default in terminals, remote desktop, password managers, File Explorer and launchers.",
          ],
          ["Hanja (F7)", "The list under the text, numbered 1–9 per page."],
        ]}
      />
      <p>
        <strong>Type English in JIS Korean.</strong> 한/영 is an on/off switch and nothing more: English typed in ABC gets
        no predictions, F9/F10 or hanja, and switching mid-word splits the word. The recommended way is to stay in JIS
        Korean (Mode B on a US ANSI keyboard), let the predictions read English words, and train them on your own words
        with a training file (<Source path="docs/training-contract.md" />) at{" "}
        <code>%APPDATA%\JIS Korean\training.json</code>.
      </p>

      <h2>Settings</h2>
      <p>
        In the indicator's menu, stored in <code>HKCU\Software\JIS Korean</code>:
      </p>
      <Table
        code
        head={["Value", "Default", "What"]}
        rows={[
          ["Mode", "0", "0 Mode A (JIS), 1 Mode B (US ANSI), 2 Mode C (UK ISO)"],
          ["EnglishMode", "unset", "ABC's layout: an English mode's id, 0xFFFFFFFF for Windows' layout, unset for the keyboard's default"],
          ["DedicatedSymbols", "1", "Mode C's [ ] and ISO extra key"],
          ["Predictions", "1", "The predictive layer"],
          ["EnterCommits", "0", "Enter commits without a new line"],
          ["LearnFromChoices", "0", "Learning from your choices, stays on this PC"],
          ["Apps\\<program.exe>\\Predictions", "", "Predictions in one app"],
          ["Apps\\<program.exe>\\Off", "", "1: every key passes through in that program"],
          ["DiagnosticLog", "0", "Activations, focus and errors, never typed text, to %ProgramData%\\JIS Korean\\jiskorean.log"],
        ]}
      />

      <h2>Passwords and screen recording</h2>
      <ul>
        <li>The list never appears in screen recordings, screenshots or screen sharing.</li>
        <li>Password boxes that say they are one get no input method at all.</li>
        <li>
          Password boxes that don't (common in banking and certificate programs) compose like text: use ABC there, or the
          menu's <strong>Off in &lt;program&gt;</strong>.
        </li>
      </ul>

      <h2>Known limits</h2>
      <ul>
        <li>Password fields and the secure desktop (UAC, sign-in) don't use input processors.</li>
        <li>Apps that read raw keys (games) see Windows' own layout.</li>
        <li>Browsers take F10 for their menu: use Shift+Z.</li>
      </ul>
      <p>
        More in <Source path="windows/README.md" />, and why the shell does what it does with TSF in{" "}
        <Source path="docs/troubleshooting/windows-tsf-notes.md" />.
      </p>
    </>
  );
}
