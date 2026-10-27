import ApplicationServices
import Carbon
import Foundation
import IOKit.hid

#if ENGLISH
/// The keyboard read at the hardware level (USB HID usages), for the one key macOS loses: the Windows
/// JIS カタカナ key (usage 0x88), which macOS discards (or on some setups turns into かな) (spec
/// §8.2.1). A press goes to Korean; whether it's held is kept for the key tap. Only keys from
/// keyboards configured as JIS (Keyboard Setup Assistant's type for each keyboard) count: カタカナ
/// means nothing on the others. Every keyboard is opened, and the JIS ones are read again from
/// macOS's settings when a Japanese key arrives, so a keyboard plugged in or configured as JIS
/// later counts without a restart. Needs Input Monitoring permission, which it asks for only when
/// `ask` (the input menu).
enum HardwareKeys {
    private static var manager: IOHIDManager?
    fileprivate static var katakanaDown = false
    fileprivate static var katakanaReleased = Date.distantPast
    /// The real かな key (LANG_JA, usage 0x90), so a かな event made of it is never taken for カタカナ.
    fileprivate static var kanaDown = false
    fileprivate static var kanaReleased = Date.distantPast
    private static var polling = false

    /// Opens the keyboards if Input Monitoring is granted. `ask`: asks for it first if it isn't,
    /// then tries again every few seconds until it is. Otherwise never asks (opening keyboards
    /// without the permission would).
    static func start(ask: Bool = false) {
        guard manager == nil else { return }
        let access = IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)
        if access != kIOHIDAccessTypeGranted {
            if ask {
                _ = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)
            }
            note("waiting: Input Monitoring \(access.rawValue) (0 granted, 1 denied, 2 unknown), Accessibility \(AXIsProcessTrusted())")
            if ask || polling {
                retry()
            }
            return
        }
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
        IOHIDManagerSetDeviceMatching(manager, [
            kIOHIDDeviceUsagePageKey: kHIDPage_GenericDesktop, kIOHIDDeviceUsageKey: kHIDUsage_GD_Keyboard,
        ] as CFDictionary)
        IOHIDManagerRegisterInputValueCallback(manager, { _, _, _, value in
            let element = IOHIDValueGetElement(value)
            let usage = IOHIDElementGetUsage(element)
            // Only the Japanese keys (0x87–0x94: ろ, カタカナ, ¥, 変換, 無変換, かな, 英数) matter.
            guard IOHIDElementGetUsagePage(element) == UInt32(kHIDPage_KeyboardOrKeypad), (0x87...0x94).contains(usage),
                  HardwareKeys.isJIS(IOHIDElementGetDevice(element))
            else { return }
            let down = IOHIDValueGetIntegerValue(value) != 0
            DiagnosticLog.note(String(format: "hardware key 0x%02X %@", usage, down ? "down" : "up"))
            if usage == UInt32(kHIDUsage_KeyboardLANG1) {  // 0x90, かな
                if !down { HardwareKeys.kanaReleased = Date() }
                HardwareKeys.kanaDown = down
                return
            }
            guard usage == UInt32(kHIDUsage_KeyboardInternational2) else { return }  // 0x88, カタカナ
            if !down { HardwareKeys.katakanaReleased = Date() }
            // macOS discards カタカナ (no app or tap sees it) or turns it into かな: either way, the
            // press itself goes to Korean, here.
            if down && !HardwareKeys.katakanaDown {
                DispatchQueue.main.async { KoreanKeyTap.goToKorean(katakana: true) }
            }
            HardwareKeys.katakanaDown = down
        }, nil)
        IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
        let status = IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone))
        // Exclusive access: a keyboard another program holds (Karabiner-Elements seizes the ones it
        // remaps) didn't open, but the others did. Only "not permitted" means waiting.
        guard status == kIOReturnSuccess || status == kIOReturnExclusiveAccess else {
            IOHIDManagerUnscheduleFromRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
            note("waiting: open \(status), Input Monitoring \(IOHIDCheckAccess(kIOHIDRequestTypeListenEvent).rawValue), Accessibility \(AXIsProcessTrusted())")
            if ask || polling {
                retry()
            }
            return
        }
        polling = false
        readJIS()
        note("on; \(jis.count) keyboard(s) configured as JIS\(status == kIOReturnSuccess ? "" : ", some held by another program")")
        self.manager = manager
    }

    /// Whether any keyboard is configured as JIS.
    static func hasJISKeyboard() -> Bool {
        readJIS()
        return !jis.isEmpty
    }

    private static func retry() {
        polling = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { start() }
    }

    private static var lastState = ""

    private static func note(_ state: String) {
        if state != lastState {
            DiagnosticLog.note("hardware keys: \(state)")
            lastState = state
        }
    }

    private struct Keyboard: Hashable {
        let vendor: Int
        let product: Int
    }

    /// The keyboards macOS has configured as JIS, and when they were read.
    private static var jis = Set<Keyboard>()
    private static var jisRead = Date.distantPast

    /// Whether a key's keyboard is configured as JIS. The list is read again (at most once a
    /// second) when a key comes from one that isn't on it: it may have just been configured.
    fileprivate static func isJIS(_ device: IOHIDDevice) -> Bool {
        let number = { (key: String) in (IOHIDDeviceGetProperty(device, key as CFString) as? NSNumber)?.intValue }
        guard let vendor = number(kIOHIDVendorIDKey), let product = number(kIOHIDProductIDKey) else { return false }
        let keyboard = Keyboard(vendor: vendor, product: product)
        if !jis.contains(keyboard) && Date().timeIntervalSince(jisRead) > 1 {
            readJIS()
        }
        return jis.contains(keyboard)
    }

    /// /Library/Preferences/com.apple.keyboardtype lists each keyboard ("product-vendor-country")
    /// with its keyboard type.
    private static func readJIS() {
        jisRead = Date()
        let types = (UserDefaults(suiteName: "/Library/Preferences/com.apple.keyboardtype")?
            .dictionary(forKey: "keyboardtype") as? [String: Int]) ?? [:]
        var keyboards = Set<Keyboard>()
        for (device, type) in types where KBGetLayoutType(Int16(type)) == OSType(kKeyboardJIS) {
            let parts = device.split(separator: "-").compactMap { Int($0) }
            if parts.count >= 2 {
                keyboards.insert(Keyboard(vendor: parts[1], product: parts[0]))
            }
        }
        if keyboards != jis {
            DiagnosticLog.note("hardware keys: \(keyboards.count) keyboard(s) configured as JIS")
        }
        jis = keyboards
    }

    /// Whether a かな event is カタカナ's: カタカナ is held (for a key down) or was just let go (for its
    /// key up: the release may be seen first), and the real かな key isn't. With both held, かな is
    /// the real one: LANG_JA works while LANG_KO is down.
    static func katakanaHeld(release: Bool) -> Bool {
        let katakana = katakanaDown || (release && Date().timeIntervalSince(katakanaReleased) < 0.3)
        let kana = kanaDown || (release && Date().timeIntervalSince(kanaReleased) < 0.3)
        return katakana && !kana
    }
}
#endif
