import Cocoa
import InputMethodKit

private let enterCommitsDefault = "EnterCommits"
/// Dedicated symbol keys (spec §3.6): Mode C's [ ] and ISO extra key. On by default.
private let symbolKeysDefault = "DedicatedSymbolKeys"
#if ENGLISH
/// The Switch Key (spec §8.2.1): a virtual key code that toggles between JIS English and the last
/// Korean mode. F18 by default; -1: none.
private let switchKeyDefault = "SwitchKey"
/// Shift+X in JIS English goes back to the last Korean mode (spec §8.2.1). On by default.
private let shiftXDefault = "ShiftXReturnsToKorean"

private func registerSwitchDefaults() {
    UserDefaults.standard.register(defaults: [switchKeyDefault: kVK_F18, shiftXDefault: true])
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
        #if ENGLISH
        if let handled = handleSwitchKey(event, client) {
            return handled
        }
        #endif
        #if PREDICT
        if let handled = handleList(event, client) {
            return handled
        }
        // A key that commits takes the reading on screen, not the Korean one underneath.
        if shown != nil && commits(event) {
            commitPrediction(chosen ?? 0, to: client)
            if event.keyCode == UInt16(kVK_Space) && modifiers(event.modifierFlags) & ~UInt32(JK_MOD_SHIFT) == 0 {
                client.insertText(" ", replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
                return true
            }
        }
        chosen = nil
        #endif
        // The auto-repeat of a held key, and Caps Lock: JIS English's press and hold and F9/F10 read
        // them (spec §3.6.3, §3.6.4); the Korean modes ignore them.
        var extra = event.isARepeat ? UInt32(JK_MOD_REPEAT) : 0
        if event.modifierFlags.contains(.capsLock) { extra |= UInt32(JK_MOD_CAPS) }
        let consumed = jk_key(engine, engineKey(event.keyCode), modifiers(event.modifierFlags) | extra)
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
        if tag == kTextServiceInputModePropertyTag, let id = value as? String, let mode = InputModes.mode(sourceID: id) {
            applyMode(mode, client: sender as? IMKTextInput)
        }
        super.setValue(value, forTag: tag, client: sender)
    }

    private func applyMode(_ mode: InputModes.Mode, client: IMKTextInput?) {
        InputModes.becameCurrent(mode)
        jk_set_mode(engine, mode.id)
        KeyboardViewerLayout.select(mode.keyboardViewerLayout, for: client)
    }

    override func deactivateServer(_ sender: Any!) {
        flush(sender)
        SuggestionWindow.shared.hide()
        super.deactivateServer(sender)
    }

    /// Called by the system on focus changes, mouse clicks in the text and similar.
    override func commitComposition(_ sender: Any!) {
        flush(sender)
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
        // The Switch Key (spec §8.2.1); any other key code can be set with
        // `defaults write me.sjun.inputmethod.JISKorean SwitchKey -int <code>`.
        let switching = NSMenuItem(title: "Switch Key (JIS English ↔ Korean)", action: nil, keyEquivalent: "")
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
        let shiftX = NSMenuItem(title: "Shift+X in JIS English Returns to Korean", action: #selector(toggleShiftX(_:)), keyEquivalent: "")
        shiftX.state = UserDefaults.standard.bool(forKey: shiftXDefault) ? .on : .off
        menu.addItem(shiftX)
        #endif
        // The public API's one setting (spec §8.2): reading the mode is always open, changing it isn't.
        let allow = NSMenuItem(title: "Let Other Apps Change the Mode", action: #selector(toggleAllowModeChange(_:)), keyEquivalent: "")
        allow.state = PublicAPI.allowed ? .on : .off
        menu.addItem(allow)
        return menu
    }

    #if ENGLISH
    @objc func setSwitchKey(_ sender: Any?) {
        guard let code = (sender as? NSMenuItem)?.tag else { return }
        UserDefaults.standard.set(code, forKey: switchKeyDefault)
    }

    @objc func toggleShiftX(_ sender: Any?) {
        registerSwitchDefaults()
        UserDefaults.standard.set(!UserDefaults.standard.bool(forKey: shiftXDefault), forKey: shiftXDefault)
    }

    /// 英数, the Switch Key and Shift+X (spec §8.2.1): switch between JIS English and the last Korean
    /// mode, inside this input method. Nil: not a switch key here, or the mode to switch to isn't
    /// added in System Settings (then the key goes on as usual, and macOS may act on it). かな is left
    /// to macOS, which selects Japanese.
    private func handleSwitchKey(_ event: NSEvent, _ client: IMKTextInput) -> Bool? {
        let mods = modifiers(event.modifierFlags)
        guard let english = InputModes.english.first, mods & UInt32(JK_MOD_CTRL | JK_MOD_ALT | JK_MOD_META) == 0
        else { return nil }
        registerSwitchDefaults()
        let defaults = UserDefaults.standard
        let code = Int(event.keyCode)
        let inEnglish = !InputModes.current.korean
        let target: InputModes.Mode
        if code == defaults.integer(forKey: switchKeyDefault) {
            target = inEnglish ? InputModes.lastKorean : english
        } else if inEnglish && code == kVK_ANSI_X && mods & UInt32(JK_MOD_SHIFT) != 0 && defaults.bool(forKey: shiftXDefault) {
            // Only in JIS English: in the Korean modes Shift+X is ㅌ (the core principle).
            target = InputModes.lastKorean
        } else if code == kVK_JIS_Eisu {
            // Already English: nothing to do, and ABC isn't wanted.
            if inEnglish { return true }
            target = english
        } else {
            return nil
        }
        guard InputModes.isEnabled(target) else { return nil }
        if !event.isARepeat {
            flush(client)
            InputModes.select(target)
        }
        return true
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
            commitPrediction(chosen ?? 0, to: client)
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
            shown = chosen.map { predictions[$0] } ?? (topIsKorean ? nil : predictions.first)
        } else {
            chosen = nil
            shown = nil
        }
        #endif
        refresh(client)
    }

    /// The marked text and the suggestion list, from the current state.
    private func refresh(_ client: IMKTextInput) {
        let notFound = NSRange(location: NSNotFound, length: NSNotFound)
        var preedit = text(jk_preedit_ptr(engine), jk_preedit_len(engine))
        #if PREDICT
        if let chosen, chosen < predictions.count {
            shown = predictions[chosen]
        }
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
        var caret = NSRect.zero
        _ = client.attributes(forCharacterIndex: 0, lineHeightRectangle: &caret)
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
            window.show(items, selected: selected, below: caret)
            return
        }
        #if PREDICT
        if !empty && predictions.count > 1 {
            window.onPick = { [weak self] index in self?.commitPrediction(index, to: client) }
            window.show(predictions, selected: chosen, below: caret)
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
