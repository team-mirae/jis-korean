import Cocoa

#if PREDICT
/// Apps where predictions start off (spec §10.11): typing there is commands, paths, passwords or a
/// game's chat, where a reading other than the keys typed does harm, or the app has its own
/// integration. Detected when JIS Korean is used in them: nothing is added to the user's settings
/// until they turn predictions on there themselves (which then wins).
enum DefaultOffApps {
    enum Reason {
        /// Commands, paths, passwords, remote sessions.
        case sensitive(String)
        /// Minecraft Java Edition: the JIS Korean mod types in-game.
        case dedicatedMod

        var notice: String {
            switch self {
            case .sensitive(let kind): "sensitive app (\(kind))"
            case .dedicatedMod: "dedicated mod recommended"
            }
        }
    }

    private static let known: [String: Reason] = {
        var map: [String: Reason] = [:]
        let groups: [(String, [String])] = [
            ("terminal", KnownApps.terminals),
            ("file transfer / SSH", [
                "com.panic.Transmit", "com.panic.transmit", "com.panic.Transmit5", "ch.sudo.cyberduck",
                "org.filezilla-project.filezilla", "de.filezilla", "com.binarynights.ForkLift",
                "com.binarynights.ForkLift-3", "com.binarynights.forklift4", "com.eltima.cmd1",
                "com.termius-dmg.mac", "com.termius.mac", "com.panic.Prompt", "com.panic.prompt3",
                "com.royalapps.royaltsx",
            ]),
            ("containers and VMs", [
                "com.docker.docker", "com.electron.dockerdesktop", "dev.orbstack.OrbStack",
                "com.parallels.desktop.console", "com.vmware.fusion", "com.utmapp.UTM", "org.virtualbox.app.VirtualBox",
            ]),
            ("remote desktop", [
                "com.microsoft.rdc.macos", "com.microsoft.rdc.mac", "com.edovia.screens.mac", "com.edovia.screens5",
                "com.realvnc.vncviewer", "com.teamviewer.TeamViewer", "com.philandro.anydesk", "com.p5sys.jump.mac.viewer",
            ]),
            ("passwords", [
                "com.1password.1password", "com.agilebits.onepassword7", "com.bitwarden.desktop",
                "org.keepassxc.keepassxc", "com.apple.Passwords", "com.apple.keychainaccess",
            ]),
            ("Finder and launchers", [
                "com.apple.finder", "com.apple.Spotlight", "com.raycast.macos", "com.runningwithcrayons.Alfred",
                "at.obdev.LaunchBar",
            ]),
        ]
        for (kind, ids) in groups {
            for id in ids { map[id.lowercased()] = .sensitive(kind) }
        }
        for id in [
            "com.mojang.minecraftlauncher", "com.mojang.launcher", "net.minecraft.launcher", "com.moonsworth.client",
            "org.prismlauncher.prismlauncher", "net.java.openjdk.java", "com.oracle.java", "org.openjdk.java",
        ] {
            map[id] = .dedicatedMod
        }
        return map
    }()

    /// Why predictions start off in this app, if they do.
    static func reason(_ bundleID: String?) -> Reason? {
        guard let id = bundleID?.lowercased() else { return nil }
        if let reason = known[id] {
            return reason
        }
        // Terminals and file-transfer apps not on the list.
        for (word, kind) in [("terminal", "terminal"), ("sftp", "file transfer / SSH"), ("ssh", "file transfer / SSH"), (".ftp", "file transfer / SSH")]
            where id.contains(word) {
            return .sensitive(kind)
        }
        return nil
    }

    /// Whether predictions are on in this app: the user's explicit setting if any, else off for the
    /// apps above, else on.
    static func predicting(_ bundleID: String?) -> Bool {
        let entry = DisabledApps.entry(bundleID)
        if entry?.noPredictions == true { return false }
        if entry?.allowPredictions == true { return true }
        return reason(bundleID) == nil
    }

    private static let noticedKey = "DefaultOffNoticed"

    /// Shows the notice the first time predictions start off in an app (once per app).
    static func noticeIfFirst(_ bundleID: String?) {
        guard let bundleID, let reason = reason(bundleID), DisabledApps.entry(bundleID) == nil else { return }
        var noticed = Set(UserDefaults.standard.stringArray(forKey: noticedKey) ?? [])
        guard !noticed.contains(bundleID) else { return }
        noticed.insert(bundleID)
        UserDefaults.standard.set(Array(noticed).sorted(), forKey: noticedKey)
        StatusNotice.show("JIS Korean: predictions are off in \(DisabledApps.displayName(bundleID)): \(reason.notice). Turn them on from the input menu.")
    }
}

/// A small notice just under the input menu's icon in the menu bar (where macOS draws it, found
/// among the on-screen windows), or the top right of the screen. It goes away on its own.
enum StatusNotice {
    private static var panel: NSPanel?

    static func show(_ text: String) {
        panel?.orderOut(nil)
        let label = NSTextField(wrappingLabelWithString: text)
        label.font = .systemFont(ofSize: 12)
        label.preferredMaxLayoutWidth = 300
        let size = label.fittingSize
        let width = min(size.width, 300) + 24, height = size.height + 16
        let panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                            styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.level = .statusBar
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.ignoresMouseEvents = false
        let background = NSVisualEffectView(frame: NSRect(x: 0, y: 0, width: width, height: height))
        background.material = .popover
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = 10
        label.frame = NSRect(x: 12, y: 8, width: width - 24, height: size.height)
        background.addSubview(label)
        panel.contentView = background
        let screen = NSScreen.main ?? NSScreen.screens[0]
        let anchor = inputMenuIcon(on: screen)
        let x = min(max((anchor?.midX ?? screen.frame.maxX - 120) - width / 2, screen.frame.minX + 8), screen.frame.maxX - width - 8)
        let y = screen.visibleFrame.maxY - height - 6
        panel.setFrameOrigin(NSPoint(x: x, y: y))
        panel.orderFrontRegardless()
        self.panel = panel
        DispatchQueue.main.asyncAfter(deadline: .now() + 7) { [weak panel] in
            panel?.orderOut(nil)
        }
    }

    /// The input menu's icon, in screen coordinates (bottom-left origin), if macOS lists it.
    private static func inputMenuIcon(on screen: NSScreen) -> NSRect? {
        guard let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] else { return nil }
        for window in windows where (window[kCGWindowOwnerName as String] as? String) == "TextInputMenuAgent" {
            guard let bounds = window[kCGWindowBounds as String] as? [String: CGFloat],
                  let x = bounds["X"], let y = bounds["Y"], let w = bounds["Width"], let h = bounds["Height"]
            else { continue }
            // CG coordinates have the origin at the top left of the main display.
            let top = NSScreen.screens[0].frame.maxY
            return NSRect(x: x, y: top - y - h, width: w, height: h)
        }
        return nil
    }
}
#endif
