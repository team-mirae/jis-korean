// Registers an input method bundle with Text Input Sources, so it shows up in System Settings →
// Keyboard → Input Sources without logging out, then waits (up to about 20 seconds) until the
// Keyboard Viewer layout bundled with it (spec §8.1) is listed: right after the bundle is copied,
// macOS can take a few seconds to pick it up.
// If the input method was enabled, it is then disabled and enabled again, as removing and re-adding
// it in System Settings would: re-registering alone doesn't reach apps that are already running, and
// they can be left unable to start it (docs/troubleshooting/macos-input-method-not-launching.md).
// Usage: swift register.swift <JISKorean.app>
import Carbon
import Foundation

guard CommandLine.arguments.count > 1 else { fatalError("usage: register.swift <app>") }
let path = CommandLine.arguments[1]
let url = URL(fileURLWithPath: path) as CFURL
let bundleID = Bundle(path: path)?.bundleIdentifier ?? "me.sjun.inputmethod.JISKorean"

func layoutListed() -> Bool {
    let query = [
        kTISPropertyBundleID as String: bundleID,
        kTISPropertyInputSourceType as String: kTISTypeKeyboardLayout as String,
    ] as CFDictionary
    return !(TISCreateInputSourceList(query, true)?.takeRetainedValue() as? [TISInputSource] ?? []).isEmpty
}

/// The input method's own sources: the bundle-level source and its input mode (Info.plist
/// ComponentInputModeDict), not the Keyboard Viewer layout. The mode is the one that gets selected.
func inputMethodSources(enabledOnly: Bool) -> [TISInputSource] {
    let query = [
        kTISPropertyBundleID as String: bundleID,
        kTISPropertyInputSourceCategory as String: kTISCategoryKeyboardInputSource as String,
    ] as CFDictionary
    let sources = TISCreateInputSourceList(query, !enabledOnly)?.takeRetainedValue() as? [TISInputSource] ?? []
    return sources.filter { source in
        guard let type = TISGetInputSourceProperty(source, kTISPropertyInputSourceType) else { return false }
        return Unmanaged<CFString>.fromOpaque(type).takeUnretainedValue() != kTISTypeKeyboardLayout
    }
}

func id(_ source: TISInputSource) -> String? {
    guard let value = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) else { return nil }
    return Unmanaged<CFString>.fromOpaque(value).takeUnretainedValue() as String
}

func isMode(_ source: TISInputSource) -> Bool {
    guard let type = TISGetInputSourceProperty(source, kTISPropertyInputSourceType) else { return false }
    return Unmanaged<CFString>.fromOpaque(type).takeUnretainedValue() == kTISTypeKeyboardInputMode
}

let wasEnabled = !inputMethodSources(enabledOnly: true).isEmpty
let wasSelected = wasEnabled
    && (id(TISCopyCurrentKeyboardInputSource().takeRetainedValue())?.hasPrefix(bundleID) ?? false)

let status = TISRegisterInputSource(url)
guard status == noErr else {
    print("TISRegisterInputSource failed: \(status)")
    exit(1)
}
for _ in 0..<20 where !layoutListed() {
    Thread.sleep(forTimeInterval: 1)
    _ = TISRegisterInputSource(url)
}
// Enabling every source of the bundle also enables the input mode when upgrading from a build
// without one, so it doesn't have to be added again in System Settings.
let sources = inputMethodSources(enabledOnly: false)
if wasEnabled, !sources.isEmpty {
    var off = noErr, on = noErr
    for source in sources { let s = TISDisableInputSource(source); if s != noErr { off = s } }
    for source in sources { let s = TISEnableInputSource(source); if s != noErr { on = s } }
    // Selecting straight after enabling can fail with paramErr until the system catches up.
    var select = noErr
    if wasSelected {
        for _ in 0..<10 {
            let enabled = inputMethodSources(enabledOnly: true)
            guard let target = enabled.first(where: isMode) ?? enabled.first else { select = OSStatus(paramErr); break }
            select = TISSelectInputSource(target)
            if select == noErr { break }
            Thread.sleep(forTimeInterval: 0.5)
        }
    }
    if off != noErr || on != noErr || select != noErr {
        print("Re-enabling the input source failed (disable \(off), enable \(on), select \(select)):")
        print("remove JIS Korean in System Settings → Keyboard → Input Sources and add it back.")
    } else {
        print("Re-enabled the input source\(wasSelected ? " and selected it again" : "").")
    }
}
print(layoutListed() ? "Registered (with its Keyboard Viewer layout)." : "Registered (the Keyboard Viewer layout will appear after logging out and in).")
