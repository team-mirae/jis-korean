import Foundation

/// The input method's own diagnostics: one line per event in ~/Library/Logs/JISKorean.log. Off
/// unless turned on with `defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true`
/// (read on each line, so it takes effect without restarting the input method).
enum DiagnosticLog {
    static let key = "DiagnosticLog"

    static func note(_ message: String) {
        guard UserDefaults.standard.bool(forKey: key) else { return }
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
