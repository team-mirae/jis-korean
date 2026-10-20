import { Code, PageHead, Source, Table } from "./Docs";

export function MacPage() {
  return (
    <>
      <PageHead title="macOS" folder="mac/">
        An InputMethodKit input method, in Swift, around the engine. It turns macOS key events into engine keys and
        shows what the engine says: inserted text, marked text, hanja candidates and the suggestion list.
      </PageHead>

      <h2>What it is</h2>
      <ul>
        <li>
          <strong>A thin shell.</strong> One controller per text field, each with its own engine. The controller moves key
          codes in and text out; it never decides what a key types.
        </li>
        <li>
          <strong>Built without Xcode.</strong> <code>make</code> builds the engine as a universal static library (arm64 and
          x86_64), compiles the Swift sources against its C header, and assembles <code>JISKorean.app</code>.{" "}
          <code>make PREDICT=0</code> builds the lite version without the predictive layer.
        </li>
        <li>
          <strong>Installed per user</strong> in <code>~/Library/Input Methods</code>. <code>make install</code> swaps the
          bundle in with renames and re-enables only the input sources that were enabled: a missing bundle, even briefly,
          leaves running apps unable to start it (<Source path="docs/troubleshooting/macos-input-method-not-launching.md">troubleshooting</Source>).
        </li>
      </ul>
      <Code lang="sh">{`
cd mac
make install        # build, install, register; then add it in System Settings
make PREDICT=0      # the lite build
`}</Code>

      <h2>Modes are input sources</h2>
      <p>
        Each engine mode is its own input mode of the one input method, the way Japanese has Kana and Romaji. Add the ones
        you want in System Settings → Keyboard → Input Sources → + → Korean.
      </p>
      <Table
        head={["Mode", "Input source ID", "Shown as"]}
        rows={[
          ["A", <code>me.sjun.inputmethod.JISKorean.2Set</code>, "JIS Korean"],
          ["B", <code>me.sjun.inputmethod.JISKorean.2SetANSI</code>, "JIS Korean for US ANSI"],
        ]}
      />
      <p>
        <strong>Why input sources, and not a setting:</strong> the mode follows the keyboard, and switching apps doesn't
        change which keyboard you're typing on, so a per-app mode would mean nothing. As input sources, the modes are
        visible to everything that understands input sources: the input menu, System Settings, and tools such as
        Hammerspoon and Karabiner, which read and select them by ID. The IDs never change once released.
      </p>
      <p>
        Each mode has its own Keyboard Viewer layout (<code>JISKorean.keylayout</code>, <code>JISKoreanANSI.keylayout</code>
        ), generated at build time from the engine's legends. Its Cmd, Ctrl and Option layers are ABC's, so shortcuts are
        unchanged.
      </p>

      <h2>Settings</h2>
      <p>In the input menu while JIS Korean is selected:</p>
      <Table
        head={["Menu item", "Stored as", "Default"]}
        rows={[
          ["Predictions", <code>Predictions</code>, "On"],
          ["Learn from My Choices (Stays on This Mac)", <code>LearnFromChoices</code>, "Off"],
          ["Learned Weights → Reset, Open Training Folder", <code>Learned</code>, "–"],
          ["Disable Predicting / Learning in this app, Manage Disabled Apps…", <code>DisabledApps</code>, "Some apps start with predictions off"],
          ["Enter Commits Without a New Line", <code>EnterCommits</code>, "Off"],
          ["Let Other Apps Change the Mode", <code>AllowOtherAppsToChangeMode</code>, "Off"],
        ]}
      />
      <p>
        And with <code>defaults write me.sjun.inputmethod.JISKorean …</code>: <code>Weight.English</code>,{" "}
        <code>Weight.Korean</code> and the other prediction weights (a multiplier, 1.0 = as learned), and{" "}
        <code>DiagnosticLog -bool true</code> for <code>~/Library/Logs/JISKorean.log</code> (off by default).
      </p>

      <h2>Public API</h2>
      <p>
        Any program can read the mode, hear it change, resolve keys and text, and, if you allowed it, change the mode. It
        works by distributed notifications, which every language can post and observe, and from the command line through
        the input method's own executable:
      </p>
      <Code lang="sh">{`
J=~/Library/"Input Methods"/JISKorean.app/Contents/MacOS/JISKorean
`}</Code>

      <h3>Read the mode</h3>
      <Code lang="sh">{`
defaults read me.sjun.inputmethod.JISKorean CurrentMode    # A or B
"$J" mode                                                  # A  JIS Korean Original Layout  me.sjun…2Set
`}</Code>
      <p>
        Also <code>CurrentModeID</code>, <code>CurrentModeInputSourceID</code> and <code>CurrentModeName</code>.{" "}
        <code>CurrentMode</code> is the mode last used; whether JIS Korean is the selected input source right now is the
        system's current input source.
      </p>

      <h3>Notifications</h3>
      <Table
        code
        head={["Name (me.sjun.inputmethod.JISKorean.…)", "Direction", "User info"]}
        rows={[
          ["ModeDidChange", "Posted by JIS Korean", <code>mode, id, inputSourceID, name</code>],
          ["SetMode", "Posted to JIS Korean", <code>mode</code>],
          ["ModeChangeRefused", "Posted by JIS Korean", <code>reason, requested</code>],
          ["ResolveKeyCodeInput", "Posted to JIS Korean", <code>requestID, keyCode, context?</code>],
          ["ResolveKeyCodeInputResult", "Posted by JIS Korean", <code>requestID, result?, app, mode, predictions, error?</code>],
          ["ResolveKeyCodeFromOutput", "Posted to JIS Korean", <code>requestID, text, minKeypressCount, maxKeypressCount</code>],
          ["ResolveKeyCodeFromOutputResult", "Posted by JIS Korean", <code>requestID, combinations, app, mode, predictions, error?</code>],
        ]}
      />

      <h3>Change the mode</h3>
      <p>
        Only when <strong>Let Other Apps Change the Mode</strong> is on. <code>mode</code> is A, B, an engine mode id or an
        input source ID. The answer is <code>ModeDidChange</code>, or <code>ModeChangeRefused</code> with the reason: the
        setting is off, the mode is unknown, or it isn't added in System Settings (the API never adds one). Selecting the
        input source directly, from the input menu or another tool, is macOS's own switching and always works.
      </p>
      <Code lang="sh">{`
"$J" mode B
`}</Code>

      <h3>Resolve keys and text</h3>
      <p>
        The engine's resolving functions (see the Engine docs), run on the engine of the app JIS Korean was last active
        in, so they match that app exactly: its mode, and its predictions setting, weights and what was learned. Every
        answer names the app, the mode and whether predictions were on. Key codes are <code>KeyboardEvent.code</code> names
        (<code>KeyG</code>) or macOS virtual key codes (<code>5</code>), with <code>S-</code> for Shift.
      </p>
      <Code lang="sh">{`
"$J" resolveKeyCodeInput KeyG           # {"result":"ㅎ","app":…,"mode":"A","predictions":true,…}
"$J" resolveKeyCodeInput KeyS 하        # after the context 하: "한"
"$J" resolveKeyCodeFromOutput ㅃ        # {"combinations":[["Digit1"],["S-KeyQ"]],…}
"$J" resolveKeyCodeFromOutput 한 3 3    # 3 presses; a nonsensical range gives []
`}</Code>

      <h3>From Hammerspoon</h3>
      <Code lang="lua">{`
local api = "me.sjun.inputmethod.JISKorean."
hs.distributednotifications.new(function(_, _, info)
  print("JIS Korean is in mode " .. info.mode)
end, api .. "ModeDidChange"):start()

hs.distributednotifications.post(api .. "SetMode", nil, { mode = "B" })
`}</Code>

      <h3>From Swift</h3>
      <Code lang="swift">{`
let center = DistributedNotificationCenter.default()
let id = UUID().uuidString
center.addObserver(forName: .init("me.sjun.inputmethod.JISKorean.ResolveKeyCodeFromOutputResult"),
                   object: nil, queue: .main) { note in
    guard note.userInfo?["requestID"] as? String == id else { return }
    print(note.userInfo?["combinations"] ?? [])
}
center.postNotificationName(.init("me.sjun.inputmethod.JISKorean.ResolveKeyCodeFromOutput"), object: nil,
    userInfo: ["requestID": id, "text": "한", "minKeypressCount": 3, "maxKeypressCount": 3],
    deliverImmediately: true)
`}</Code>
    </>
  );
}
