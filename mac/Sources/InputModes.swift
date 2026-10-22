import Carbon
import Cocoa

/// JIS Korean's modes (spec §3.6) as macOS input modes: one input source per engine mode, the way
/// Japanese has Kana and Romaji (spec §8.2).
///
/// Why input sources and not a setting: the mode follows the keyboard, and apps don't change which
/// keyboard the user is typing on, so per-app switching would mean nothing. As input sources the
/// modes are visible to everything that already understands input sources: the input menu, System
/// Settings, and tools such as Hammerspoon or Karabiner, which can read and select them by ID.
enum InputModes {
    struct Mode {
        /// The engine's mode id (`jk_set_mode`), stable across releases.
        let id: UInt32
        /// The mode name (spec §3.6.1), "A", "B", "A_QWERTY": what the spec and the public API call it.
        let letter: String
        /// The input source ID (Info.plist ComponentInputModeDict). Never rename one: users' tools
        /// select it by this ID.
        let sourceID: String
        /// The Keyboard Viewer layout bundled for it (spec §8.1).
        let keyboardViewerLayout: String
        /// Whether it types Korean. JIS English doesn't (spec §3.6.2).
        var korean = true
    }

    static let all = [
        Mode(id: 0, letter: "A", sourceID: "me.sjun.inputmethod.JISKorean.2Set", keyboardViewerLayout: "JISKorean"),
        Mode(id: 1, letter: "B", sourceID: "me.sjun.inputmethod.JISKorean.2SetANSI", keyboardViewerLayout: "JISKoreanANSI"),
        Mode(id: 2, letter: "C", sourceID: "me.sjun.inputmethod.JISKorean.2SetISO", keyboardViewerLayout: "JISKoreanISO"),
    ] + english

    #if ENGLISH
    /// The English input modes (spec §3.6.2, §3.6.4), built with `make ENGLISH=1`: JIS English, its
    /// US ANSI and UK ISO siblings, and JIS Dvorak and JIS Colemak.
    static let english = [
        Mode(id: 3, letter: "A_QWERTY", sourceID: "me.sjun.inputmethod.JISKorean.A_QWERTY",
             keyboardViewerLayout: "JISEnglish", korean: false),
        Mode(id: 9, letter: "B_QWERTY", sourceID: "me.sjun.inputmethod.JISKorean.B_QWERTY",
             keyboardViewerLayout: "USEnglish", korean: false),
        Mode(id: 10, letter: "CJ_QWERTY", sourceID: "me.sjun.inputmethod.JISKorean.CJ_QWERTY",
             keyboardViewerLayout: "UKEnglish", korean: false),
        Mode(id: 7, letter: "A_DVORAK", sourceID: "me.sjun.inputmethod.JISKorean.A_DVORAK",
             keyboardViewerLayout: "JISDvorak", korean: false),
        Mode(id: 8, letter: "A_COLEMAK", sourceID: "me.sjun.inputmethod.JISKorean.A_COLEMAK",
             keyboardViewerLayout: "JISColemak", korean: false),
    ]
    #else
    static let english: [Mode] = []
    #endif

    /// The Korean mode used last, which the Korean Key goes back to from an English mode (spec
    /// §8.2.1).
    static var lastKorean: Mode {
        UserDefaults.standard.string(forKey: lastKoreanKey).flatMap(mode(sourceID:)).flatMap { $0.korean ? $0 : nil }
            ?? defaultMode
    }

    private static let lastKoreanKey = "LastKoreanMode"
    private static let lastEnglishKey = "LastEnglishMode"

    /// The English mode 英数 goes to (spec §8.2.1): the one used last if it's added
    /// in System Settings, else the first English mode that is. Nil: none is.
    static var englishTarget: Mode? {
        let last = UserDefaults.standard.string(forKey: lastEnglishKey).flatMap(mode(sourceID:))
        return ([last].compactMap { $0 } + english).first { !$0.korean && isEnabled($0) }
    }

    /// Selects a mode's input source, as the input menu would: `noErr`, `paramErr` if it isn't added
    /// in System Settings, or macOS's own error.
    @discardableResult
    static func select(_ mode: Mode) -> OSStatus {
        let query = [kTISPropertyInputSourceID as String: mode.sourceID] as CFDictionary
        guard let source = (TISCreateInputSourceList(query, false)?.takeRetainedValue() as? [TISInputSource])?.first else {
            return OSStatus(paramErr)
        }
        return TISSelectInputSource(source)
    }

    /// Whether a mode's input source is added in System Settings.
    static func isEnabled(_ mode: Mode) -> Bool {
        let query = [kTISPropertyInputSourceID as String: mode.sourceID] as CFDictionary
        return (TISCreateInputSourceList(query, false)?.takeRetainedValue() as? [TISInputSource])?.isEmpty == false
    }

    static let defaultMode = all[0]

    /// The mode last selected, shared by every input controller.
    private(set) static var current = defaultMode

    static func mode(sourceID: String) -> Mode? {
        all.first { $0.sourceID == sourceID }
    }

    /// A mode named by its letter ("B"), engine id ("1") or input source ID.
    static func mode(named name: String) -> Mode? {
        all.first { $0.letter.caseInsensitiveCompare(name) == .orderedSame || String($0.id) == name || $0.sourceID == name }
    }

    /// The engine's name for a mode ("JIS Korean for US ANSI").
    static func name(_ mode: Mode) -> String {
        guard let pointer = jk_mode_name_ptr(mode.id) else { return mode.letter }
        return String(decoding: UnsafeBufferPointer(start: pointer, count: jk_mode_name_len(mode.id)), as: UTF8.self)
    }

    /// The mode of the input source that's selected now, if it's one of ours.
    static func selectedNow() -> Mode? {
        let source = TISCopyCurrentKeyboardInputSource().takeRetainedValue()
        guard let value = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) else { return nil }
        return mode(sourceID: Unmanaged<CFString>.fromOpaque(value).takeUnretainedValue() as String)
    }

    /// Records a mode the user switched to, and tells other programs if it changed (PublicAPI).
    static func becameCurrent(_ mode: Mode) {
        let changed = mode.sourceID != current.sourceID || UserDefaults.standard.string(forKey: PublicAPI.currentModeKey) != mode.letter
        current = mode
        UserDefaults.standard.set(mode.sourceID, forKey: mode.korean ? lastKoreanKey : lastEnglishKey)
        if changed {
            PublicAPI.publish(mode)
        }
    }
}

/// JIS Korean's public API for other programs (spec §8.2, §8.3, mac/README.md): read the current
/// mode, hear when it changes, ask for a change, and resolve keys and text. Changing the mode only
/// works when the user has turned on "Let Other Apps Change the Mode" in the input menu, with the
/// other settings; everything else only reads.
///
/// - Read: `defaults read me.sjun.inputmethod.JISKorean CurrentMode` → A, B or C (also `CurrentModeID`,
///   `CurrentModeInputSourceID`, `CurrentModeName`), or `JISKorean.app/Contents/MacOS/JISKorean mode`.
/// - Changes: the distributed notification `me.sjun.inputmethod.JISKorean.ModeDidChange`, with
///   `mode`, `id`, `inputSourceID` and `name` in its user info.
/// - Change: post `me.sjun.inputmethod.JISKorean.SetMode` with `mode` (A, B, an engine id or an input
///   source ID) in its user info, or run `JISKorean mode B`. JIS Korean answers with ModeDidChange,
///   or with `…ModeChangeRefused` and a `reason`.
/// - Resolve: `…ResolveKeyCodeInput` (`keyCode`, optional `context`) and `…ResolveKeyCodeFromOutput`
///   (`text`, `minKeypressCount`, `maxKeypressCount`), each with a `requestID`, answered by the same
///   name + `Result`; or `JISKorean resolveKeyCodeInput …` / `resolveKeyCodeFromOutput …`.
enum PublicAPI {
    static let prefix = "me.sjun.inputmethod.JISKorean."
    static let modeDidChange = Notification.Name(prefix + "ModeDidChange")
    static let setMode = Notification.Name(prefix + "SetMode")
    static let modeChangeRefused = Notification.Name(prefix + "ModeChangeRefused")

    static let currentModeKey = "CurrentMode"
    /// The setting that lets other programs change the mode (off by default).
    static let allowKey = "AllowOtherAppsToChangeMode"

    static var allowed: Bool { UserDefaults.standard.bool(forKey: allowKey) }

    /// Starts listening for SetMode. Call once, in the input method's process.
    static func start() {
        let center = DistributedNotificationCenter.default()
        center.addObserver(forName: setMode, object: nil, queue: .main) { note in
            handleSetMode(note.userInfo?["mode"] as? String)
        }
        center.addObserver(forName: resolveKeyCodeInput, object: nil, queue: .main) { note in
            reply(resolveKeyCodeInput, note.userInfo, resolveInput(note.userInfo ?? [:]))
        }
        center.addObserver(forName: resolveKeyCodeFromOutput, object: nil, queue: .main) { note in
            reply(resolveKeyCodeFromOutput, note.userInfo, resolveOutput(note.userInfo ?? [:]))
        }
        // A mode selected from outside (the input menu, Ctrl+Space, Karabiner, Hammerspoon) is the
        // current one even when macOS doesn't tell the open session: the fields follow it on
        // their next key.
        center.addObserver(forName: Notification.Name(kTISNotifySelectedKeyboardInputSourceChanged as String), object: nil,
                           queue: .main) { _ in
            if let mode = InputModes.selectedNow() {
                InputModes.becameCurrent(mode)
            }
        }
        if UserDefaults.standard.string(forKey: currentModeKey) == nil {
            store(InputModes.current)
        }
    }

    static func publish(_ mode: InputModes.Mode) {
        store(mode)
        DistributedNotificationCenter.default().postNotificationName(
            modeDidChange, object: nil, userInfo: info(mode), deliverImmediately: true)
        DiagnosticLog.note("mode \(mode.letter) (\(mode.sourceID))")
    }

    private static func store(_ mode: InputModes.Mode) {
        let defaults = UserDefaults.standard
        defaults.set(mode.letter, forKey: currentModeKey)
        defaults.set(Int(mode.id), forKey: "CurrentModeID")
        defaults.set(mode.sourceID, forKey: "CurrentModeInputSourceID")
        defaults.set(InputModes.name(mode), forKey: "CurrentModeName")
    }

    private static func info(_ mode: InputModes.Mode) -> [String: Any] {
        ["mode": mode.letter, "id": Int(mode.id), "inputSourceID": mode.sourceID, "name": InputModes.name(mode)]
    }

    private static func refuse(_ reason: String, _ requested: String?) {
        DistributedNotificationCenter.default().postNotificationName(
            modeChangeRefused, object: nil, userInfo: ["reason": reason, "requested": requested ?? ""], deliverImmediately: true)
        DiagnosticLog.note("refused a mode change to \(requested ?? "nothing"): \(reason)")
    }

    private static func handleSetMode(_ requested: String?) {
        guard allowed else {
            return refuse("Other apps may not change the mode: turn on “Let Other Apps Change the Mode” in the JIS Korean input menu.", requested)
        }
        guard let requested, let mode = InputModes.mode(named: requested) else {
            return refuse("Unknown mode. Use a mode name (A, B, C), an engine mode id, or an input source ID.", requested)
        }
        guard InputModes.isEnabled(mode) else {
            return refuse("\(InputModes.name(mode)) isn't added in System Settings → Keyboard → Input Sources.", requested)
        }
        let status = InputModes.select(mode)
        guard status == noErr else { return refuse("Selecting the input source failed (\(status)).", requested) }
        InputModes.becameCurrent(mode)
        // Answer even when nothing changed, so the caller always hears back.
        DistributedNotificationCenter.default().postNotificationName(
            modeDidChange, object: nil, userInfo: info(mode), deliverImmediately: true)
    }

    // MARK: Resolving keys and text (spec §8.3)

    static let resolveKeyCodeInput = Notification.Name(prefix + "ResolveKeyCodeInput")
    static let resolveKeyCodeFromOutput = Notification.Name(prefix + "ResolveKeyCodeFromOutput")

    /// A key press for the engine: a `KeyboardEvent.code` ("KeyG"), or a macOS virtual key code
    /// ("5", or the number), either with "S-" for Shift.
    static func press(_ value: Any?) -> String? {
        var text: String
        switch value {
        case let number as NSNumber: text = number.stringValue
        case let string as String: text = string.trimmingCharacters(in: .whitespaces)
        default: return nil
        }
        var shift = ""
        if text.hasPrefix("S-") || text.hasPrefix("Shift+") {
            shift = "S-"
            text = String(text.drop(while: { $0 != "-" && $0 != "+" }).dropFirst())
        }
        if let virtualKey = Int(text) {
            guard let id = keyMap[virtualKey] else { return nil }
            text = String(decoding: UnsafeBufferPointer(start: jk_key_code_ptr(id), count: jk_key_code_len(id)), as: UTF8.self)
        }
        return shift + text
    }

    /// The engine a request runs on (a copy of it is used), and the facts every answer carries.
    private static func target() -> (OpaquePointer, [String: Any])? {
        guard let controller = JKInputController.focused, let engine = controller.engine else { return nil }
        let mode = InputModes.current
        return (engine, ["app": controller.appID ?? "", "mode": mode.letter, "predictions": controller.predicting])
    }

    private static func resolved(_ engine: OpaquePointer) -> String {
        String(decoding: UnsafeBufferPointer(start: jk_resolved_ptr(engine), count: jk_resolved_len(engine)), as: UTF8.self)
    }

    private static let notActive = "JIS Korean hasn't been active in any app since it started: select it and click into a text field."

    /// resolveKeyCodeInput(keyCode[, context]) → `result`: the character the key types alone, or
    /// what the field shows after the context and the key. No `result` when there's none.
    static func resolveInput(_ info: [AnyHashable: Any]) -> [String: Any] {
        guard let (engine, facts) = target() else { return ["error": notActive] }
        guard let key = press(info["keyCode"]) else { return facts.merging(["error": "Unknown key code."]) { $1 } }
        var bytes = Array(key.utf8)
        let found: UInt32
        if let context = info["context"] as? String {
            let contextBytes = Array(context.utf8)
            found = jk_resolve_key_input_after(engine, contextBytes, contextBytes.count, &bytes, bytes.count)
        } else {
            found = jk_resolve_key_input(engine, &bytes, bytes.count)
        }
        return found == 0 ? facts : facts.merging(["result": resolved(engine)]) { $1 }
    }

    /// resolveKeyCodeFromOutput(text, min = 1, max = 1) → `combinations`: arrays of key presses.
    /// Empty for a nonsensical range, never an error.
    static func resolveOutput(_ info: [AnyHashable: Any]) -> [String: Any] {
        guard let (engine, facts) = target() else { return ["error": notActive, "combinations": [[String]]()] }
        let number = { (value: Any?) -> Int64? in
            switch value {
            case nil: return 1
            case let n as NSNumber: return n.int64Value
            case let s as String: return Int64(s.trimmingCharacters(in: .whitespaces))
            default: return nil
            }
        }
        let text = Array((info["text"] as? String ?? "").utf8)
        let (min, max) = (number(info["minKeypressCount"]) ?? 0, number(info["maxKeypressCount"]) ?? 0)
        jk_resolve_keys_for_output(engine, text, text.count, min, max)
        let json = Data(resolved(engine).utf8)
        let combinations = (try? JSONSerialization.jsonObject(with: json)) as? [[String]] ?? []
        return facts.merging(["combinations": combinations]) { $1 }
    }

    private static func reply(_ request: Notification.Name, _ info: [AnyHashable: Any]?, _ answer: [String: Any]) {
        var answer = answer
        answer["requestID"] = info?["requestID"] ?? ""
        DistributedNotificationCenter.default().postNotificationName(
            Notification.Name(request.rawValue + "Result"), object: nil, userInfo: answer, deliverImmediately: true)
    }

    /// Posts a request to the running input method and waits up to 5 seconds for its answer.
    private static func ask(_ request: Notification.Name, _ info: [String: Any]) -> [AnyHashable: Any]? {
        let id = UUID().uuidString
        var answer: [AnyHashable: Any]?
        let center = DistributedNotificationCenter.default()
        let observer = center.addObserver(forName: Notification.Name(request.rawValue + "Result"), object: nil, queue: .main) { note in
            if note.userInfo?["requestID"] as? String == id { answer = note.userInfo }
        }
        center.postNotificationName(request, object: nil, userInfo: info.merging(["requestID": id]) { $1 }, deliverImmediately: true)
        let deadline = Date().addingTimeInterval(5)
        while answer == nil, Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        }
        center.removeObserver(observer)
        return answer
    }

    private static func json(_ value: Any) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes]) else { return "null" }
        return String(decoding: data, as: UTF8.self)
    }

    /// `JISKorean mode [A|B|C]`, `JISKorean resolveKeyCodeInput <keyCode> [context]`,
    /// `JISKorean resolveKeyCodeFromOutput <text> [min] [max]`. Returns the exit status.
    static func runCommand(_ arguments: [String]) -> Int32 {
        let usage = """
            usage: JISKorean mode [A|B|C]
                   JISKorean resolveKeyCodeInput <keyCode> [context]
                   JISKorean resolveKeyCodeFromOutput <text> [minKeypressCount] [maxKeypressCount]
            """
        switch arguments.first {
        case "resolveKeyCodeInput" where (2...3).contains(arguments.count):
            var info: [String: Any] = ["keyCode": arguments[1]]
            if arguments.count == 3 { info["context"] = arguments[2] }
            guard let answer = ask(resolveKeyCodeInput, info) else { return notRunning() }
            print(json(answer))
            return answer["result"] == nil ? 1 : 0
        case "resolveKeyCodeFromOutput" where (2...4).contains(arguments.count):
            var info: [String: Any] = ["text": arguments[1]]
            if arguments.count > 2 { info["minKeypressCount"] = arguments[2] }
            if arguments.count > 3 { info["maxKeypressCount"] = arguments[3] }
            guard let answer = ask(resolveKeyCodeFromOutput, info) else { return notRunning() }
            print(json(answer))
            return answer["error"] == nil ? 0 : 1
        case "mode" where arguments.count <= 2:
            break
        default:
            print(usage)
            return 2
        }
        guard arguments.count == 2 else {
            let defaults = UserDefaults.standard
            let letter = defaults.string(forKey: currentModeKey) ?? InputModes.defaultMode.letter
            let mode = InputModes.mode(named: letter) ?? InputModes.defaultMode
            print("\(mode.letter)\t\(InputModes.name(mode))\t\(mode.sourceID)")
            return 0
        }
        var answer: (String, Bool)?
        let center = DistributedNotificationCenter.default()
        let observers = [
            center.addObserver(forName: modeDidChange, object: nil, queue: .main) { note in
                answer = ("\(note.userInfo?["mode"] ?? "?")\t\(note.userInfo?["name"] ?? "")", true)
            },
            center.addObserver(forName: modeChangeRefused, object: nil, queue: .main) { note in
                answer = ("\(note.userInfo?["reason"] ?? "refused")", false)
            },
        ]
        center.postNotificationName(setMode, object: nil, userInfo: ["mode": arguments[1]], deliverImmediately: true)
        let deadline = Date().addingTimeInterval(3)
        while answer == nil, Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        }
        observers.forEach(center.removeObserver)
        guard let answer else { return notRunning() }
        print(answer.0)
        return answer.1 ? 0 : 1
    }

    private static func notRunning() -> Int32 {
        print("No answer: JIS Korean isn't running. Select it once in the input menu, then try again.")
        return 1
    }
}
