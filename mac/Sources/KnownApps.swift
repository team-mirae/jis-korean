import Foundation

/// Apps JIS Korean treats differently, by bundle identifier (compared lower-cased).
enum KnownApps {
    /// Terminals: typing there is commands. Predictions start off there (DefaultOffApps), and ⌥ goes
    /// to the app untouched in the English modes (readline's ⌥ shortcuts, spec §3.6.3).
    static let terminals = [
        "com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable", "dev.warp.Warp",
        "com.mitchellh.ghostty", "net.kovidgoyal.kitty", "org.alacritty", "io.alacritty",
        "com.github.wez.wezterm", "co.zeit.hyper", "org.tabby", "com.raphaelamorim.rio",
    ]

    static func isTerminal(_ bundleID: String?) -> Bool {
        guard let id = bundleID?.lowercased() else { return false }
        return terminals.contains { $0.lowercased() == id } || id.contains("terminal")
    }
}

