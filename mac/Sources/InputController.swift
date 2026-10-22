import Cocoa
import InputMethodKit
import IOKit.hid

private let enterCommitsDefault = "EnterCommits"
/// Dedicated symbol keys (spec §3.6): Mode C's [ ] and ISO extra key. On by default.
private let symbolKeysDefault = "DedicatedSymbolKeys"
#if ENGLISH
/// The Korean Key (spec §8.2.1): a virtual key code that always selects Korean: from an English
/// mode the last Korean mode, in a Korean mode that mode again (so macOS shows it under the caret).
/// F18 by default; -1: none.
private let switchKeyDefault = "KoreanKey"

private func registerSwitchDefaults() {
    UserDefaults.standard.register(defaults: [switchKeyDefault: kVK_F18])
}
#endif
#if PREDICT
private let predictionsDefault = "Predictions"
/// Learning from the user's choices (spec §10.11): opt-in, stays on this Mac.
private let learningDefault = "LearnFromChoices"
/// What was learned: one line of counts, no text or keys.
private let learnedDefault = "Learned"
private let weightNames = ["Korean", "English", "Implied", "Mixed", "TwoSet", "KeepFinals", "Hanja", "ImpliedL"]
#endif

/// One controller per text input client; each owns its own engine. All behavior is in the engine
/// (docs/spec.md): this class only moves key events in and text out, and shows the suggestion list.
@objc(JKInputController)
final class JKInputController: IMKInputController {
    /// This controller's engine. The public API resolves keys on a copy of it (spec §8.3).
    let engine = jk_new()
    private var hasMarkedText = false
    /// Characters before the insertion point the next marked text replaces (`jk_take_back`): the
    /// letter JIS English's press and hold takes back into its list (spec §3.6.3).
    private var takeBack = 0
    /// The controller JIS Korean was last activated in: the focused app's, while JIS Korean is the
    /// input source. The public API answers for it.
    private(set) static weak var focused: JKInputController?
    /// The controller that last received a key: the field really being typed in. Chrome activates
    /// its fields (the omnibox, the page) often and out of order, so the last activated one isn't
    /// always it.
    private(set) static weak var keyed: JKInputController?
    /// Unmapped key codes already logged, per app (see `handle`).
    private static var unmappedNoted = Set<String>()
    /// The mode this controller's engine is in (`jk_set_mode`), nil before the first.
    private var modeID: UInt32?
    /// The app it types into, and whether predictions are on there (spec §10.11).
    private(set) var appID: String?
    private(set) var predicting = false
    #if PREDICT
    /// The ranked readings of what's staged (spec §10), shown in the suggestion list.
    private var predictions: [String] = []
    /// The row chosen with ↓ / Tab, or nil while nothing is chosen (the top reading applies).
    private var chosen: Int?
    /// The reading shown as the marked text in place of the Korean one: the chosen row, or the top
    /// prediction when it isn't the Korean reading. Nil: the engine's own preedit shows.
    private var shown: String?
    /// What shows while no row is chosen: the top prediction when it isn't the Korean reading.
    private var topShown: String?
    /// The bundle identifier of the app this controller types into (for the per-app settings).
    private var clientApp: String?
    #endif

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        jk_set_dictionary(engine, sharedDictionary)
        #if PREDICT
        UserDefaults.standard.register(defaults: [predictionsDefault: true, learningDefault: false])
        applyPredictionsSetting()
        #endif
    }

    deinit {
        jk_free(engine)
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else { return false }
        Self.keyed = self
        syncMode(client)
        if engineKey(event.keyCode) == UInt32(JK_KEY_OTHER) {
            // Keys JIS Korean doesn't map, with the app: shows which keys an app passes on at all
            // (iTerm keeps F1–F19 to itself) and what code a remapped key arrives as. Once per code
            // and app, so what's typed on unmapped keys (a PIN on the keypad) isn't in the log.
            let note = "unmapped key code \(event.keyCode) in \(client.bundleIdentifier() ?? "?")"
            if Self.unmappedNoted.insert(note).inserted {
                DiagnosticLog.note(note)
            }
        }
        #if ENGLISH
        if let handled = handleSwitchKey(event, client) {
            return handled
        }
        #endif
        // The auto-repeat of a held key, and Caps Lock: JIS English's press and hold and F9/F10 read
        // them (spec §3.6.3, §3.6.4); the Korean modes ignore them.
        var extra = event.isARepeat ? UInt32(JK_MOD_REPEAT) : 0
        if event.modifierFlags.contains(.capsLock) { extra |= UInt32(JK_MOD_CAPS) }
        let mods = modifiers(event.modifierFlags) | extra
        #if PREDICT
        if let handled = handleList(event, client) {
            return handled
        }
        // A key that commits takes the reading on screen, not the Korean one underneath, and then
        // does to it what it does to any text (Space adds a space, Shift+Enter only commits).
        if shown != nil && commits(event) {
            let consumed = jk_key_after_prediction(engine, chosen ?? 0, engineKey(event.keyCode), mods)
            chosen = nil
            apply(to: client)
            saveLearned()
            return consumed != 0
        }
        chosen = nil
        #endif
        let consumed = jk_key(engine, engineKey(event.keyCode), mods)
        apply(to: client)
        #if PREDICT
        saveLearned()
        #endif
        return consumed != 0
    }

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        appID = (sender as? IMKTextInput)?.bundleIdentifier()
        Self.focused = self
        DiagnosticLog.note("activated in \((sender as? IMKTextInput)?.bundleIdentifier() ?? "?")")
        #if PREDICT
        clientApp = (sender as? IMKTextInput)?.bundleIdentifier()
        DefaultOffApps.noticeIfFirst(clientApp)
        #endif
        jk_activate(engine)
        // Which mode's input source was selected (spec §8.2). setValue(_:forTag:client:) also
        // reports it, but not always before the first key.
        applyMode(InputModes.selectedNow() ?? InputModes.current, client: sender as? IMKTextInput)
        #if PREDICT
        applyPredictionsSetting()
        // A new or changed training file (docs/training-contract.md): applied, then settings again.
        Training.applyIfChanged(to: engine)
        saveLearned()
        applyPredictionsSetting()
        #endif
        jk_set_enter_commits(engine, UserDefaults.standard.bool(forKey: enterCommitsDefault) ? 1 : 0)
        applySymbolKeysSetting()
    }

    private func applySymbolKeysSetting() {
        UserDefaults.standard.register(defaults: [symbolKeysDefault: true])
        jk_set_dedicated_symbols(engine, UserDefaults.standard.bool(forKey: symbolKeysDefault) ? 1 : 0)
    }

    /// The system names the input mode the user picked: JIS Korean's modes are input modes.
    override func setValue(_ value: Any!, forTag tag: Int, client sender: Any!) {
        // Switching commits what's unconfirmed first (spec §8.2.1): applyMode does it.
        if tag == kTextServiceInputModePropertyTag, let id = value as? String, let mode = InputModes.mode(sourceID: id) {
            applyMode(mode, client: sender as? IMKTextInput)
        }
        super.setValue(value, forTag: tag, client: sender)
    }

    private func applyMode(_ mode: InputModes.Mode, client: IMKTextInput?) {
        InputModes.becameCurrent(mode)
        setEngineMode(mode, client: client)
    }

    /// Puts this controller's engine and its client's keyboard layout on `mode`, committing what
    /// the old mode left unconfirmed.
    private func setEngineMode(_ mode: InputModes.Mode, client: IMKTextInput?) {
        if let modeID, modeID != mode.id {
            flush(client)
        }
        jk_set_mode(engine, mode.id)
        modeID = mode.id
        KeyboardViewerLayout.select(mode.keyboardViewerLayout, for: client)
    }

    /// Keeps this field's engine on the mode JIS Korean is in. A switch can reach another
    /// controller than the one typing (カタカナ and F18 from the key tap, the public API, a
    /// selection macOS didn't report to this session); this field then catches up on its next
    /// key. Otherwise its engine stays in the old mode, and an English engine passes letters on
    /// to the Korean mode's layout, which types them as loose jamo (ㄷㅏㄹㅏ for 다락방).
    private func syncMode(_ client: IMKTextInput) {
        let mode = InputModes.current
        guard modeID != mode.id else { return }
        DiagnosticLog.note("mode resync \(modeID.map { String($0) } ?? "none") -> \(mode.letter) in \(client.bundleIdentifier() ?? "?")")
        setEngineMode(mode, client: client)
    }

    override func deactivateServer(_ sender: Any!) {
        noteEarlyCommit("deactivateServer", sender)
        flush(sender)
        SuggestionWindow.shared.hide()
        super.deactivateServer(sender)
    }

    /// Called by the system on focus changes, mouse clicks in the text and similar.
    override func commitComposition(_ sender: Any!) {
        noteEarlyCommit("commitComposition", sender)
        flush(sender)
    }

    /// Logs a commit the app or the system asked for while something was unconfirmed: how many
    /// characters, never the text.
    private func noteEarlyCommit(_ what: String, _ sender: Any!) {
        let length = jk_preedit_len(engine)
        guard length > 0 else { return }
        let app = (sender as? IMKTextInput)?.bundleIdentifier() ?? appID ?? "?"
        DiagnosticLog.note("\(what) in \(app) with \(length) unconfirmed byte(s)")
    }

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        #if PREDICT
        let predictions = NSMenuItem(title: "Predictions", action: #selector(togglePredictions(_:)), keyEquivalent: "")
        predictions.state = UserDefaults.standard.bool(forKey: predictionsDefault) ? .on : .off
        menu.addItem(predictions)
        let learning = NSMenuItem(title: "Learn from My Choices (Stays on This Mac)", action: #selector(toggleLearning(_:)), keyEquivalent: "")
        learning.state = UserDefaults.standard.bool(forKey: learningDefault) ? .on : .off
        menu.addItem(learning)
        // What was learned, read-only: one line per scenario that moved.
        let learned = NSMenuItem(title: "Learned Weights", action: nil, keyEquivalent: "")
        let submenu = NSMenu()
        let labels = ["Korean", "English", "Implied ㅇ", "English + particle", "Plain 2-set", "Finals kept", "Hanja", "Implied ㄹ after ㄴ"]
        for (id, label) in labels.enumerated() {
            let value = jk_learned_weight(engine, UInt32(id))
            let item = NSMenuItem(title: String(format: "%@  ×%.2f", label, value), action: nil, keyEquivalent: "")
            item.isEnabled = false
            submenu.addItem(item)
        }
        submenu.addItem(.separator())
        submenu.addItem(NSMenuItem(title: "Reset What It Learned", action: #selector(resetLearned(_:)), keyEquivalent: ""))
        submenu.addItem(NSMenuItem(title: "Open Training Folder", action: #selector(openTrainingFolder(_:)), keyEquivalent: ""))
        learned.submenu = submenu
        menu.addItem(learned)
        // Per app (spec §10.11): what this app does, and the list of excluded apps.
        menu.addItem(.separator())
        let app = clientApp ?? (client() as IMKTextInput?)?.bundleIdentifier()
        if let app {
            let name = DisabledApps.displayName(app)
            let entry = DisabledApps.entry(app)
            let learning = NSMenuItem(title: entry?.noLearning == true ? "Resume Learning from \(name)" : "Disable Learning from \(name)",
                                      action: #selector(toggleAppLearning(_:)), keyEquivalent: "")
            learning.representedObject = app
            menu.addItem(learning)
            let on = DefaultOffApps.predicting(app)
            let why = DefaultOffApps.reason(app).map { " (off by default: \($0.notice))" } ?? ""
            let predicting = NSMenuItem(title: on ? "Disable Predicting in \(name)" : "Resume Predicting in \(name)\(why)",
                                        action: #selector(toggleAppPredictions(_:)), keyEquivalent: "")
            predicting.representedObject = app
            menu.addItem(predicting)
        }
        menu.addItem(NSMenuItem(title: "Manage Disabled Apps…", action: #selector(manageDisabledApps(_:)), keyEquivalent: ""))
        #endif
        let item = NSMenuItem(title: "Enter Commits Without a New Line", action: #selector(toggleEnterCommits(_:)), keyEquivalent: "")
        item.state = UserDefaults.standard.bool(forKey: enterCommitsDefault) ? .on : .off
        menu.addItem(item)
        // Only Mode C has optional symbol keys (spec §3.6).
        if InputModes.current.id == 2 {
            let symbols = NSMenuItem(title: "Dedicated Symbol Keys ([ ] and the Key Left of Z)", action: #selector(toggleSymbolKeys(_:)), keyEquivalent: "")
            UserDefaults.standard.register(defaults: [symbolKeysDefault: true])
            symbols.state = UserDefaults.standard.bool(forKey: symbolKeysDefault) ? .on : .off
            menu.addItem(symbols)
        }
        #if ENGLISH
        // The Korean Key (spec §8.2.1); any other key code can be set with
        // `defaults write me.sjun.inputmethod.JISKorean KoreanKey -int <code>`.
        let switching = NSMenuItem(title: "Korean Key (Back to Korean)", action: nil, keyEquivalent: "")
        let keys = NSMenu()
        registerSwitchDefaults()
        let current = UserDefaults.standard.integer(forKey: switchKeyDefault)
        for (title, code) in [("None", -1), ("F13", kVK_F13), ("F14", kVK_F14), ("F15", kVK_F15), ("F16", kVK_F16),
                              ("F17", kVK_F17), ("F18", kVK_F18), ("F19", kVK_F19)] {
            let key = NSMenuItem(title: title, action: #selector(setSwitchKey(_:)), keyEquivalent: "")
            key.tag = code
            key.state = code == current ? .on : .off
            keys.addItem(key)
        }
        switching.submenu = keys
        menu.addItem(switching)
        // カタカナ and the Korean Key everywhere (iTerm, other input sources) need two permissions.
        if !AXIsProcessTrusted() || IOHIDCheckAccess(kIOHIDRequestTypeListenEvent) != kIOHIDAccessTypeGranted {
            menu.addItem(NSMenuItem(title: "Allow カタカナ and F18 Everywhere…", action: #selector(askForKeyPermissions(_:)), keyEquivalent: ""))
        }
        #endif
        // The public API's one setting (spec §8.2): reading the mode is always open, changing it isn't.
        let allow = NSMenuItem(title: "Let Other Apps Change the Mode", action: #selector(toggleAllowModeChange(_:)), keyEquivalent: "")
        allow.state = PublicAPI.allowed ? .on : .off
        menu.addItem(allow)
        return menu
    }

    #if ENGLISH
    @objc func askForKeyPermissions(_ sender: Any?) {
        KoreanKeyTap.start(ask: true)
    }

    @objc func setSwitchKey(_ sender: Any?) {
        guard let code = (sender as? NSMenuItem)?.tag else { return }
        UserDefaults.standard.set(code, forKey: switchKeyDefault)
    }

    /// 英数 and the Korean Key (spec §8.2.1): 英数 always goes to English, the Korean Key always to
    /// Korean (the last Korean mode), inside this input method. Nil: not a switch key here, or the mode to switch to isn't
    /// added in System Settings (then the key goes on as usual, and macOS may act on it). かな is left
    /// to macOS, which selects Japanese.
    private func handleSwitchKey(_ event: NSEvent, _ client: IMKTextInput) -> Bool? {
        let mods = modifiers(event.modifierFlags)
        guard !InputModes.english.isEmpty, mods & UInt32(JK_MOD_CTRL | JK_MOD_ALT | JK_MOD_META) == 0
        else { return nil }
        registerSwitchDefaults()
        let defaults = UserDefaults.standard
        let code = Int(event.keyCode)
        let inEnglish = !InputModes.current.korean
        let target: InputModes.Mode?
        if code == defaults.integer(forKey: switchKeyDefault) {
            // In a Korean mode it selects that mode again: macOS shows the input source under the
            // caret, confirming Korean. Nothing unconfirmed is committed then.
            if !inEnglish {
                if !event.isARepeat { selectLater(InputModes.current) }
                return true
            }
            target = InputModes.lastKorean
        } else if code == kVK_JIS_Eisu {
            // Already English: nothing to do, and ABC isn't wanted.
            if inEnglish { return true }
            target = InputModes.englishTarget
        } else {
            return nil
        }
        guard let target, InputModes.isEnabled(target) else { return nil }
        if !event.isARepeat {
            // The field being typed in switches now, in place: its engine and keyboard layout. Asking
            // macOS first left it on the old mode (the open session isn't always told) and cost a
            // round trip; macOS is told afterwards, for the menu bar and the next field.
            flush(client)
            applyMode(target, client: client)
            selectLater(target)
        }
        return true
    }

    /// The Korean Key caught by the key tap (KoreanKeyTap, spec §8.2.1), in the field this
    /// controller types into: from an English mode to the last Korean mode, in place; in a Korean
    /// mode that mode again, so macOS shows it under the caret.
    func koreanKeyFromTap() {
        guard let client = client() as IMKTextInput? else { return }
        if InputModes.current.korean && modeID == InputModes.current.id {
            selectLater(InputModes.current)
            return
        }
        // English, or this field's engine didn't follow an earlier switch: Korean, in place.
        let target = InputModes.current.korean ? InputModes.current : InputModes.lastKorean
        guard InputModes.isEnabled(target) else { return }
        applyMode(target, client: client)
        selectLater(target)
    }

    /// Selects `mode`'s input source once the key being handled is done: selecting it from inside
    /// handle(_:client:) can cut the open session off (Terminal then gets the keys itself).
    private func selectLater(_ mode: InputModes.Mode) {
        DispatchQueue.main.async { InputModes.select(mode) }
    }
    #endif

    @objc func toggleAllowModeChange(_ sender: Any?) {
        UserDefaults.standard.set(!PublicAPI.allowed, forKey: PublicAPI.allowKey)
    }

    @objc func toggleSymbolKeys(_ sender: Any?) {
        UserDefaults.standard.register(defaults: [symbolKeysDefault: true])
        UserDefaults.standard.set(!UserDefaults.standard.bool(forKey: symbolKeysDefault), forKey: symbolKeysDefault)
        applySymbolKeysSetting()
    }

    @objc func toggleEnterCommits(_ sender: Any?) {
        let on = !UserDefaults.standard.bool(forKey: enterCommitsDefault)
        UserDefaults.standard.set(on, forKey: enterCommitsDefault)
        jk_set_enter_commits(engine, on ? 1 : 0)
    }

    #if PREDICT
    @objc func togglePredictions(_ sender: Any?) {
        UserDefaults.standard.set(!UserDefaults.standard.bool(forKey: predictionsDefault), forKey: predictionsDefault)
        applyPredictionsSetting()
    }

    @objc func toggleLearning(_ sender: Any?) {
        UserDefaults.standard.set(!UserDefaults.standard.bool(forKey: learningDefault), forKey: learningDefault)
        applyPredictionsSetting()
    }

    @objc func toggleAppLearning(_ sender: Any?) {
        guard let app = (sender as? NSMenuItem)?.representedObject as? String ?? clientApp else { return }
        DisabledApps.update(app) { $0.noLearning.toggle() }
        applyPredictionsSetting()
    }

    @objc func toggleAppPredictions(_ sender: Any?) {
        guard let app = (sender as? NSMenuItem)?.representedObject as? String ?? clientApp else { return }
        let defaultOff = DefaultOffApps.reason(app) != nil
        if DefaultOffApps.predicting(app) {
            // Turning off: back to the default where it starts off, else an explicit "off".
            DisabledApps.update(app) {
                $0.allowPredictions = false
                $0.noPredictions = !defaultOff
            }
        } else {
            DisabledApps.update(app) {
                $0.noPredictions = false
                $0.allowPredictions = defaultOff
            }
        }
        applyPredictionsSetting()
    }

    @objc func manageDisabledApps(_ sender: Any?) {
        DisabledAppsWindow.shared.show()
    }

    @objc func openTrainingFolder(_ sender: Any?) {
        Training.openFolder()
    }

    @objc func resetLearned(_ sender: Any?) {
        jk_learner_reset(engine)
        UserDefaults.standard.removeObject(forKey: learnedDefault)
    }

    /// On: the shared predictor. Off: the deterministic lite behavior. Also applies the user's
    /// tuning (spec §10.5), e.g. `defaults write me.sjun.inputmethod.JISKorean Weight.English -float 0.8`
    /// (`Weight.ImpliedL -float 0` turns the implied ㄹ after ㄴ off)
    /// (a weight set this way wins over the learned one), and what was learned (spec §10.11).
    private func applyPredictionsSetting() {
        let defaults = UserDefaults.standard
        // Per app (spec §10.11): no predictions, or no learning recorded, where the user said so.
        let app = DisabledApps.entry(clientApp)
        let predicting = defaults.bool(forKey: predictionsDefault) && DefaultOffApps.predicting(clientApp)
        let predictor = predicting ? Training.predictor() : nil
        jk_set_predictor(engine, predictor)
        self.predicting = predictor != nil
        jk_set_recording(engine, app?.noLearning == true ? 0 : 1)
        for (id, name) in weightNames.enumerated() {
            if let value = defaults.object(forKey: "Weight.\(name)") as? Double {
                jk_set_prediction_weight(engine, UInt32(id), value)
            } else {
                jk_clear_prediction_weight(engine, UInt32(id))
            }
        }
        // The latest of what every engine learned: each saves after a change, each loads here.
        if let learned = defaults.string(forKey: learnedDefault) {
            let bytes = Array(learned.utf8)
            _ = jk_learner_import(engine, bytes, bytes.count)
        } else {
            jk_learner_reset(engine)
        }
        jk_set_learning(engine, defaults.bool(forKey: learningDefault) ? 1 : 0)
    }

    /// Stores what was learned when it changed: counts only, no text or keys.
    private func saveLearned() {
        guard jk_learner_changed(engine) != 0 else { return }
        let length = jk_learner_export(engine)
        UserDefaults.standard.set(text(jk_learner_ptr(engine), length), forKey: learnedDefault)
    }

    /// Keys for the suggestion list, while it shows predictions: ↓ / Tab and ↑ move through it
    /// (the chosen row replaces the marked text), Escape leaves it, Space / Enter commit the chosen
    /// row. Nil: not a list key, handle as usual.
    private func handleList(_ event: NSEvent, _ client: IMKTextInput) -> Bool? {
        guard predictions.count > 1, jk_candidate_count(engine) == 0 else { return nil }
        let mods = modifiers(event.modifierFlags)
        guard mods & UInt32(JK_MOD_CTRL | JK_MOD_ALT | JK_MOD_META) == 0 else { return nil }
        let count = predictions.count
        let shift = mods & UInt32(JK_MOD_SHIFT) != 0
        let key = Int(event.keyCode)
        if key == kVK_DownArrow || (key == kVK_Tab && !shift) {
            chosen = ((chosen ?? -1) + 1) % count
        } else if key == kVK_UpArrow || (key == kVK_Tab && shift) {
            chosen = ((chosen ?? 0) - 1 + count) % count
        } else if key == kVK_Escape && chosen != nil {
            chosen = nil
        } else if (key == kVK_Return || key == kVK_ANSI_KeypadEnter) && chosen != nil {
            commitPrediction(chosen ?? 0, to: client)
            return true
        } else {
            return nil
        }
        refresh(client)
        return true
    }

    /// Keys whose engine handling commits the staged text.
    private func commits(_ event: NSEvent) -> Bool {
        let shortcut = modifiers(event.modifierFlags) & UInt32(JK_MOD_CTRL | JK_MOD_ALT | JK_MOD_META) != 0
        let committing: Set<Int> = [kVK_Space, kVK_Return, kVK_ANSI_KeypadEnter, kVK_Tab, kVK_Escape,
                                    kVK_LeftArrow, kVK_RightArrow, kVK_UpArrow, kVK_DownArrow]
        return shortcut || committing.contains(Int(event.keyCode)) || engineKey(event.keyCode) == UInt32(JK_KEY_OTHER)
    }

    private func commitPrediction(_ index: Int, to client: IMKTextInput) {
        jk_commit_prediction(engine, index)
        chosen = nil
        apply(to: client)
        saveLearned()
    }
    #endif

    private func flush(_ sender: Any!) {
        let client = (sender as? IMKTextInput) ?? (client() as IMKTextInput?)
        #if PREDICT
        if shown != nil, let client {
            // Committed because the system or the app asked: not a choice to learn from.
            jk_flush_prediction(engine, chosen ?? 0)
            chosen = nil
            apply(to: client)
            return
        }
        #endif
        jk_flush(engine)
        if let client {
            apply(to: client)
        }
    }

    /// Inserts the engine's commit text, then shows its preedit (or the chosen reading) as marked
    /// text, and updates the suggestion list.
    private func apply(to client: IMKTextInput) {
        let commit = text(jk_commit_ptr(engine), jk_commit_len(engine))
        if !commit.isEmpty {
            // Inserting replaces any marked text.
            client.insertText(commit, replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
            hasMarkedText = false
        }
        takeBack = jk_take_back(engine)
        #if PREDICT
        predictions = []
        let preedit = text(jk_preedit_ptr(engine), jk_preedit_len(engine))
        if !preedit.isEmpty && jk_candidate_count(engine) == 0 {
            let count = jk_predict(engine)
            predictions = (0..<count).map { text(jk_prediction_ptr(engine, $0), jk_prediction_len(engine, $0)) }
            let topIsKorean = count == 0 || jk_prediction_kind(engine, 0) == UInt32(JK_PREDICTION_KOREAN)
            if let chosen, chosen >= count { self.chosen = nil }
            topShown = topIsKorean ? nil : predictions.first
        } else {
            chosen = nil
            topShown = nil
        }
        #endif
        refresh(client)
    }

    /// The marked text and the suggestion list, from the current state.
    private func refresh(_ client: IMKTextInput) {
        let notFound = NSRange(location: NSNotFound, length: NSNotFound)
        var preedit = text(jk_preedit_ptr(engine), jk_preedit_len(engine))
        #if PREDICT
        // The chosen row, else the top reading: Escape out of a row shows (and commits) the top
        // reading again, not the row left behind.
        shown = chosen.flatMap { $0 < predictions.count ? predictions[$0] : nil } ?? topShown
        if let shown {
            preedit = shown
        }
        #endif
        if !preedit.isEmpty {
            let marked = NSAttributedString(string: preedit, attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
                .markedClauseSegment: 0,
            ])
            let caret = NSRange(location: preedit.utf16.count, length: 0)
            // Taking back characters already typed: the marked text replaces them. They're single
            // Latin letters (one UTF-16 unit each), so the count is the range's length.
            var replacement = notFound
            if takeBack > 0 {
                let selection = client.selectedRange()
                if selection.location != NSNotFound && selection.location >= takeBack {
                    replacement = NSRange(location: selection.location - takeBack, length: takeBack)
                }
                takeBack = 0
            }
            client.setMarkedText(marked, selectionRange: caret, replacementRange: replacement)
            hasMarkedText = true
        } else if hasMarkedText {
            client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: notFound)
            hasMarkedText = false
        }
        showList(client, empty: preedit.isEmpty)
    }

    /// Hanja candidates after F7, or the predictions while typing (when there's more than one).
    private func showList(_ client: IMKTextInput, empty: Bool) {
        // The caret's rectangle is a synchronous round trip into the app: asked only when a list
        // shows, not on every key.
        func caret() -> NSRect {
            var rect = NSRect.zero
            _ = client.attributes(forCharacterIndex: 0, lineHeightRectangle: &rect)
            return rect
        }
        let window = SuggestionWindow.shared
        let hanja = jk_candidate_count(engine)
        if hanja > 0 {
            let items = (0..<hanja).map { i -> String in
                let hanja = text(jk_candidate_ptr(engine, i), jk_candidate_len(engine, i))
                let comment = text(jk_candidate_comment_ptr(engine, i), jk_candidate_comment_len(engine, i))
                return comment.isEmpty ? hanja : "\(hanja)  \(comment)"
            }
            let selected = jk_candidate_selected(engine)
            window.onPick = { [weak self] index in self?.pickHanja(index, from: selected, client) }
            window.show(items, selected: selected, below: caret())
            return
        }
        #if PREDICT
        if !empty && predictions.count > 1 {
            window.onPick = { [weak self] index in self?.commitPrediction(index, to: client) }
            window.show(predictions, selected: chosen, below: caret())
            return
        }
        #endif
        window.hide()
    }

    /// A click on a hanja candidate: move the engine's selection there, then pick it.
    private func pickHanja(_ index: Int, from selected: Int, _ client: IMKTextInput) {
        let key = UInt32(index > selected ? JK_KEY_ARROWDOWN : JK_KEY_ARROWUP)
        for _ in 0..<abs(index - selected) {
            jk_key(engine, key, 0)
        }
        jk_key(engine, UInt32(JK_KEY_ENTER), 0)
        apply(to: client)
    }

    private func text(_ pointer: UnsafePointer<UInt8>?, _ length: Int) -> String {
        guard let pointer, length > 0 else { return "" }
        return String(decoding: UnsafeBufferPointer(start: pointer, count: length), as: UTF8.self)
    }

    private func modifiers(_ flags: NSEvent.ModifierFlags) -> UInt32 {
        var bits: UInt32 = 0
        if flags.contains(.shift) { bits |= UInt32(JK_MOD_SHIFT) }
        if flags.contains(.control) { bits |= UInt32(JK_MOD_CTRL) }
        if flags.contains(.option) { bits |= UInt32(JK_MOD_ALT) }
        if flags.contains(.command) { bits |= UInt32(JK_MOD_META) }
        return bits
    }
}
