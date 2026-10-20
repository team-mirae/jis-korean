import Foundation
import InputMethodKit

/// The layouts macOS uses for JIS Korean's keys (spec §8.1), one per mode: Keyboard Viewer shows
/// their legends, and apps get their characters for keys the engine passes through (the Cmd, Ctrl
/// and Option layers are ABC's, so shortcuts are unchanged). Mode A's is JISKorean.keylayout and
/// Mode B's JISKoreanANSI.keylayout, in the input method's Resources; each activation names the
/// mode's layout to the client, which finds it in this bundle (as Apple's Korean input method does
/// with 2SetHangul).
enum KeyboardViewerLayout {
    /// Names `name` (one mode's bundled layout, InputModes.Mode.keyboardViewerLayout) to the client.
    static func select(_ name: String, for client: IMKTextInput?) {
        guard let client else {
            DiagnosticLog.note("Keyboard Viewer: no client to name the layout to")
            return
        }
        client.overrideKeyboard(withKeyboardNamed: name)
        DiagnosticLog.note("Keyboard Viewer: named layout \(name) to the client (\(type(of: client)))")
    }
}
