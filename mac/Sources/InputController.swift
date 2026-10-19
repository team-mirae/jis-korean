import Cocoa
import InputMethodKit

private let enterCommitsDefault = "EnterCommits"

/// One controller per text input client; each owns its own engine. All behavior is in the engine
/// (docs/spec.md): this class only moves key events in and text out.
@objc(JKInputController)
final class JKInputController: IMKInputController {
    private let engine = jk_new()
    private var hasMarkedText = false

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        jk_set_dictionary(engine, sharedDictionary)
    }

    deinit {
        jk_free(engine)
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else { return false }
        let consumed = jk_key(engine, engineKey(event.keyCode), modifiers(event.modifierFlags))
        apply(to: client)
        return consumed != 0
    }

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        jk_activate(engine)
        jk_set_enter_commits(engine, UserDefaults.standard.bool(forKey: enterCommitsDefault) ? 1 : 0)
    }

    override func deactivateServer(_ sender: Any!) {
        flush(sender)
        super.deactivateServer(sender)
    }

    /// Called by the system on focus changes, mouse clicks in the text and similar.
    override func commitComposition(_ sender: Any!) {
        flush(sender)
    }

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        let item = NSMenuItem(title: "Enter Commits Without a New Line", action: #selector(toggleEnterCommits(_:)), keyEquivalent: "")
        item.state = UserDefaults.standard.bool(forKey: enterCommitsDefault) ? .on : .off
        menu.addItem(item)
        return menu
    }

    @objc func toggleEnterCommits(_ sender: Any?) {
        let on = !UserDefaults.standard.bool(forKey: enterCommitsDefault)
        UserDefaults.standard.set(on, forKey: enterCommitsDefault)
        jk_set_enter_commits(engine, on ? 1 : 0)
    }

    private func flush(_ sender: Any!) {
        jk_flush(engine)
        if let client = (sender as? IMKTextInput) ?? (client() as IMKTextInput?) {
            apply(to: client)
        }
    }

    /// Inserts the engine's commit text, then shows its preedit as marked text.
    private func apply(to client: IMKTextInput) {
        let notFound = NSRange(location: NSNotFound, length: NSNotFound)
        let commit = text(jk_commit_ptr(engine), jk_commit_len(engine))
        let preedit = text(jk_preedit_ptr(engine), jk_preedit_len(engine))
        if !commit.isEmpty {
            // Inserting replaces any marked text.
            client.insertText(commit, replacementRange: notFound)
            hasMarkedText = false
        }
        if !preedit.isEmpty {
            let marked = NSAttributedString(string: preedit, attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
                .markedClauseSegment: 0,
            ])
            let caret = NSRange(location: preedit.utf16.count, length: 0)
            client.setMarkedText(marked, selectionRange: caret, replacementRange: notFound)
            hasMarkedText = true
        } else if hasMarkedText {
            client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: notFound)
            hasMarkedText = false
        }
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
