import Cocoa
import InputMethodKit

#if ENGLISH
/// The keys that go to Korean, caught system-wide (spec §8.2.1), before any app sees them:
///
/// - **カタカナ**, the Windows JIS keyboards' カタカナ/ひらがな key (right of the space bar, where Korean
///   keyboards have 한/영). macOS turns it into かな (key code 104) and loses the difference, so the
///   keyboard is also read at the hardware level (HardwareKeys): a かな event while カタカナ is held
///   is カタカナ. A real かな key goes through, to Japanese.
/// - **The Korean Key** (F18 by default), for keys remapped to it (Karabiner). Some apps keep
///   function keys to themselves (iTerm keeps F1–F19), so it's caught here too.
///
/// The tap needs Accessibility permission and the hardware listener Input Monitoring; JIS Korean
/// asks for both. The tap swallows the key (down and up, repeats included) and goes to Korean: in
/// the field JIS Korean is typing in if it's the selected input source, else by selecting the last
/// Korean mode (from Japanese or ABC, say). Keys with Ctrl, Option or Cmd held go through untouched.
enum KoreanKeyTap {
    private static var tap: CFMachPort?
    private static var started = false
    /// Set once JIS Korean has asked for the permissions: it doesn't ask again on its own (the input
    /// menu's "Allow カタカナ and F18 Everywhere…" asks again).
    static let askedKey = "AskedForKeyPermissions"

    /// Starts the tap once Accessibility is granted, checking every few seconds until it is. Asks
    /// for the permissions the first time only, or when `ask` (from the input menu).
    static func start(ask: Bool = false) {
        let defaults = UserDefaults.standard
        let prompt = ask || !defaults.bool(forKey: askedKey)
        if prompt { defaults.set(true, forKey: askedKey) }
        if !started || ask {
            started = true
            HardwareKeys.start(ask: prompt)
        }
        guard tap == nil else { return }
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: prompt] as CFDictionary
        guard AXIsProcessTrustedWithOptions(options) else {
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { start() }
            return
        }
        let mask = (1 << CGEventType.keyDown.rawValue) | (1 << CGEventType.keyUp.rawValue)
        guard let port = CGEvent.tapCreate(tap: .cgSessionEventTap, place: .headInsertEventTap, options: .defaultTap,
                                           eventsOfInterest: CGEventMask(mask), callback: callback, userInfo: nil)
        else {
            DiagnosticLog.note("Korean Key tap: couldn't create the tap")
            return
        }
        tap = port
        CFRunLoopAddSource(CFRunLoopGetMain(), CFMachPortCreateRunLoopSource(nil, port, 0), .commonModes)
        CGEvent.tapEnable(tap: port, enable: true)
        DiagnosticLog.note("Korean Key tap: on")
    }

    private static let callback: CGEventTapCallBack = { _, type, event, _ in
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            if let tap { CGEvent.tapEnable(tap: tap, enable: true) }
            return Unmanaged.passUnretained(event)
        }
        let code = Int(event.getIntegerValueField(.keyboardEventKeycode))
        let key = UserDefaults.standard.object(forKey: "KoreanKey") as? Int ?? kVK_F18
        let katakana = code == kVK_JIS_Kana && HardwareKeys.katakanaHeld(release: type == .keyUp)
        if code == kVK_JIS_Kana || code == kVK_JIS_Eisu || code == key {
            DiagnosticLog.note("tap: key code \(code) \(type == .keyDown ? "down" : "up"), カタカナ held \(katakana)")
        }
        guard code == key || katakana, event.flags.intersection([.maskCommand, .maskControl, .maskAlternate]).isEmpty
        else {
            return Unmanaged.passUnretained(event)
        }
        // カタカナ already went to Korean from the hardware listener: a かな made of it is only swallowed.
        if !katakana && type == .keyDown && event.getIntegerValueField(.keyboardEventAutorepeat) == 0 {
            DispatchQueue.main.async { goToKorean() }
        }
        return nil  // swallowed: no app sees it
    }

    static func goToKorean() {
        if InputModes.selectedNow() != nil, let controller = JKInputController.focused {
            controller.koreanKeyFromTap()
        } else {
            // Another input source is selected (Japanese, ABC): select the last Korean mode.
            InputModes.select(InputModes.lastKorean)
        }
        DiagnosticLog.note("Korean Key (tap)")
    }
}
#endif
