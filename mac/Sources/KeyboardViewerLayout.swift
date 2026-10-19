import Foundation
import InputMethodKit

/// The layout macOS uses for JIS Korean's keys (spec §8.1): Keyboard Viewer shows its legends, and
/// apps get its characters for keys the engine passes through (its Cmd, Ctrl and Option layers are
/// ABC's, so shortcuts are unchanged). It's JISKorean.keylayout in the input method's Resources;
/// each activation names it to the client, which finds it in this bundle (as Apple's Korean input
/// method does with 2SetHangul).
enum KeyboardViewerLayout {
    /// The `name` of the bundled layout.
    static let name = "JISKorean"

    static func select(for client: IMKTextInput?) {
        guard let client else {
            note("Keyboard Viewer: no client to name the layout to")
            return
        }
        client.overrideKeyboard(withKeyboardNamed: name)
        note("Keyboard Viewer: named layout \(name) to the client (\(type(of: client)))")
    }

    /// One line in ~/Library/Logs/JISKorean.log (the input method's own diagnostics).
    static func note(_ message: String) {
        let url = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Logs/JISKorean.log")
        let line = "\(ISO8601DateFormatter().string(from: Date())) [\(ProcessInfo.processInfo.processIdentifier)] \(message)\n"
        if let handle = try? FileHandle(forWritingTo: url) {
            handle.seekToEndOfFile()
            handle.write(line.data(using: .utf8)!)
            try? handle.close()
        } else {
            try? line.write(to: url, atomically: true, encoding: .utf8)
        }
    }
}
