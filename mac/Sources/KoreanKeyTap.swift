import Cocoa
import InputMethodKit
import IOKit.hid

#if ENGLISH
/// The language keys, caught system-wide (spec §8.2.1), before any app sees them:
///
/// - **カタカナ**, the Windows JIS keyboards' カタカナ/ひらがな key (right of the space bar, where Korean
///   keyboards have 한/영). macOS turns it into かな (key code 104) and loses the difference, so the
///   keyboard is also read at the hardware level (HardwareKeys): a かな event while カタカナ is held
///   is カタカナ. A real かな key goes through, to Japanese. It goes to JIS Korean (Mode A).
/// - **The Korean Key** (F18 by default), for keys remapped to it (Karabiner). Some apps keep
///   function keys to themselves (iTerm keeps F1–F19), so it's caught here too. It goes to the last
///   Korean mode.
/// - **英数**, which goes to JIS English. Left to macOS, it selects Apple's ABC when ABC is enabled
///   (#2), not JIS English.
///
/// The tap needs Accessibility permission and the hardware listener Input Monitoring. JIS Korean
/// never asks for them on its own: only the input menu's "Allow カタカナ and F18 Everywhere…" does.
/// Both run only when they're of use (an English mode is added, or a keyboard is configured as
/// JIS) and the permissions are granted. The tap swallows a key (down and up, repeats included)
/// only when the mode it goes to is added, and switches: in the field JIS Korean is typing in if
/// it's the selected input source, else by selecting the mode (from Japanese or ABC, say). Keys with
/// Ctrl, Option or Cmd held go through untouched.
enum KoreanKeyTap {
    private static var tap: CFMachPort?
    private static var polling = false
    /// Whether a Korean / an English mode of JIS Korean is added in System Settings, refreshed when
    /// the input sources change: the tap's callback reads these instead of asking macOS per key.
    private static var koreanAdded = false
    private static var englishAdded = false
    private static var lastCheck = Date.distantPast
    private static var lastUseful: Bool?
    private static let notices = EnabledSourcesNotices()

    /// Starts the hardware listener and the tap if they're of use and their permissions are
    /// granted, without asking. At launch, when the added input sources change, and now and then on
    /// activation (`checkSoon`): a keyboard configured as JIS later, or a permission granted in
    /// System Settings, counts then.
    static func check() {
        notices.observe()
        lastCheck = Date()
        refreshAdded()
        let useful = englishAdded || (koreanAdded && HardwareKeys.hasJISKeyboard())
        if useful != lastUseful {
            lastUseful = useful
            DiagnosticLog.note(useful ? "Korean Key tap: wanted (permissions: Accessibility \(AXIsProcessTrusted()), Input Monitoring \(IOHIDCheckAccess(kIOHIDRequestTypeListenEvent) == kIOHIDAccessTypeGranted))"
                                      : "Korean Key tap: not needed (no English mode added, no keyboard configured as JIS)")
        }
        if useful {
            start(ask: false)
        }
    }

    /// `check`, at most every 30 seconds.
    static func checkSoon() {
        guard Date().timeIntervalSince(lastCheck) > 30 else { return }
        check()
    }

    static func refreshAdded() {
        let ids = InputModes.enabledSourceIDs()
        koreanAdded = InputModes.all.contains { $0.korean && ids.contains($0.sourceID) }
        englishAdded = InputModes.all.contains { !$0.korean && ids.contains($0.sourceID) }
    }

    /// Starts the hardware listener and the tap. `ask` (from the input menu): asks for the
    /// permissions, and checks every few seconds until they're granted. Otherwise starts only what's
    /// already allowed, and never asks.
    static func start(ask: Bool) {
        if ask {
            refreshAdded()
        }
        HardwareKeys.start(ask: ask)
        guard tap == nil else { return }
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: ask] as CFDictionary
        guard AXIsProcessTrustedWithOptions(options) else {
            if ask && !polling {
                polling = true
                poll()
            }
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

    /// After asking: waits for Accessibility, without asking again.
    private static func poll() {
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            guard tap == nil else {
                polling = false
                return
            }
            if AXIsProcessTrusted() {
                polling = false
                start(ask: false)
            } else {
                poll()
            }
        }
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
        guard event.flags.intersection([.maskCommand, .maskControl, .maskAlternate]).isEmpty else {
            return Unmanaged.passUnretained(event)
        }
        let press = type == .keyDown && event.getIntegerValueField(.keyboardEventAutorepeat) == 0
        if code == key && koreanAdded {
            if press { DispatchQueue.main.async { goToKorean(katakana: false) } }
        } else if katakana && koreanAdded {
            // カタカナ already went to Korean from the hardware listener: the かな made of it is only
            // swallowed.
        } else if code == kVK_JIS_Eisu && englishAdded {
            if press { DispatchQueue.main.async { goToEnglish() } }
        } else {
            // Nothing to go to: the key goes on as usual (to the app, or to macOS).
            return Unmanaged.passUnretained(event)
        }
        return nil  // swallowed: no app sees it
    }

    /// The Korean Key (`katakana` false) or カタカナ: to Korean, from anywhere.
    static func goToKorean(katakana: Bool) {
        // The field being typed in (JKInputController.typing). Any other field follows on its next key.
        if InputModes.selectedNow() != nil, let controller = JKInputController.typing {
            let current = InputModes.current
            let target = katakana ? InputModes.katakanaTarget : (current.korean ? current : InputModes.koreanTarget)
            if let target { controller.switchFromTap(to: target) }
        } else if let target = katakana ? InputModes.katakanaTarget : InputModes.koreanTarget {
            // Another input source is selected (Japanese, ABC): select the mode, and record it, so
            // the fields follow even if macOS doesn't tell their sessions.
            InputModes.becameCurrent(target)
            InputModes.select(target)
        }
        DiagnosticLog.note(katakana ? "カタカナ (tap)" : "Korean Key (tap)")
    }

    /// 英数: to JIS English, from anywhere.
    static func goToEnglish() {
        guard let target = InputModes.eisuTarget else { return }
        if InputModes.selectedNow() != nil, let controller = JKInputController.typing {
            controller.switchFromTap(to: target)
        } else {
            InputModes.becameCurrent(target)
            InputModes.select(target)
        }
        DiagnosticLog.note("英数 (tap)")
    }
}

/// macOS's notice that input sources were added or removed (a selector needs an object).
private final class EnabledSourcesNotices: NSObject {
    private var observing = false

    func observe() {
        guard !observing else { return }
        observing = true
        // Delivered even while suspended, as PublicAPI's notices are: an input method is never the
        // active app.
        DistributedNotificationCenter.default().addObserver(
            self, selector: #selector(changed(_:)),
            name: Notification.Name(kTISNotifyEnabledKeyboardInputSourcesChanged as String), object: nil,
            suspensionBehavior: .deliverImmediately)
    }

    @objc func changed(_ note: Notification) {
        KoreanKeyTap.check()
    }
}
#endif
