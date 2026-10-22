import Foundation
import InputMethodKit

/// The layouts macOS uses for JIS Korean's keys (spec §8.1), one per mode: Keyboard Viewer shows
/// their legends, and apps get their characters for keys the engine passes on (the Cmd, Ctrl
/// and Option layers are ABC's, so shortcuts are unchanged). Mode A's is JISKorean.keylayout,
/// Mode B's JISKoreanANSI.keylayout and Mode C's JISKoreanISO.keylayout, in the input method's
/// Resources; each activation names the mode's layout to the client, which finds it in this bundle
/// (as Apple's Korean input method does with 2SetHangul).
///
/// The layout named is the app's, not the field's: every field of the app (Chrome's omnibox and
/// its pages) is typed through the layout named last, and macOS drops it when the input method is
/// deactivated there. So the name last given to each app is kept, and a field whose mode's layout
/// isn't it names it again before its key (InputController.syncMode): otherwise an English engine's
/// letters, passed on to the app, would be typed by a Korean layout as bare jamo.
enum KeyboardViewerLayout {
    /// The layout last named to each app, by bundle identifier, since the input method was last
    /// activated there.
    private static var named: [String: String] = [:]

    /// Names `name` (one mode's bundled layout, InputModes.Mode.keyboardViewerLayout) to the client.
    static func select(_ name: String, for client: IMKTextInput?) {
        guard let client else {
            DiagnosticLog.note("Keyboard Viewer: no client to name the layout to")
            return
        }
        client.overrideKeyboard(withKeyboardNamed: name)
        let app = client.bundleIdentifier()
        if let app {
            named[app] = name
        }
        DiagnosticLog.note("Keyboard Viewer: named layout \(name) to \(app ?? "?")")
    }

    /// The layout last named to `app`, if any.
    static func named(to app: String?) -> String? {
        app.flatMap { named[$0] }
    }

    /// The input method was deactivated in `app`: macOS forgets the layout, and so does this.
    static func forget(_ app: String?) {
        if let app {
            named[app] = nil
        }
    }
}
