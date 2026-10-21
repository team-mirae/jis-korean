import ApplicationServices
import Carbon
import Foundation
import IOKit.hid

#if ENGLISH
/// The keyboard read at the hardware level (USB HID usages), for the one key macOS loses: the Windows
/// JIS カタカナ key (usage 0x88), which macOS discards (or on some setups turns into かな) (spec
/// §8.2.1). A press goes to Korean; whether it's held is kept for the key tap. Only keyboards configured as JIS (Keyboard Setup Assistant's type for each keyboard) are
/// opened: カタカナ means nothing on the others. Needs Input Monitoring permission, which it asks for.
enum HardwareKeys {
    private static var manager: IOHIDManager?
    fileprivate static var katakanaDown = false
    fileprivate static var katakanaReleased = Date.distantPast

    static func start(ask: Bool = false) {
        guard manager == nil else { return }
        if ask && IOHIDCheckAccess(kIOHIDRequestTypeListenEvent) != kIOHIDAccessTypeGranted {
            _ = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)
        }
        let keyboards = jisKeyboards()
        guard !keyboards.isEmpty else {
            DiagnosticLog.note("hardware keys: no keyboard is configured as JIS; カタカナ isn't listened for")
            return
        }
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
        IOHIDManagerSetDeviceMatchingMultiple(manager, keyboards.map {
            [kIOHIDVendorIDKey: $0.vendor, kIOHIDProductIDKey: $0.product,
             kIOHIDDeviceUsagePageKey: kHIDPage_GenericDesktop, kIOHIDDeviceUsageKey: kHIDUsage_GD_Keyboard]
        } as CFArray)
        IOHIDManagerRegisterInputValueCallback(manager, { _, _, _, value in
            let element = IOHIDValueGetElement(value)
            let usage = IOHIDElementGetUsage(element)
            guard IOHIDElementGetUsagePage(element) == UInt32(kHIDPage_KeyboardOrKeypad) else { return }
            let down = IOHIDValueGetIntegerValue(value) != 0
            // The Japanese keys (0x87–0x94: ろ, カタカナ, ¥, 変換, 無変換, かな, 英数), for the log.
            if (0x87...0x94).contains(usage) {
                DiagnosticLog.note(String(format: "hardware key 0x%02X %@", usage, down ? "down" : "up"))
            }
            guard usage == UInt32(kHIDUsage_KeyboardInternational2) else { return }  // 0x88, カタカナ
            if !down { HardwareKeys.katakanaReleased = Date() }
            // macOS discards カタカナ (no app or tap sees it) or turns it into かな: either way, the
            // press itself goes to Korean, here.
            if down && !HardwareKeys.katakanaDown {
                DispatchQueue.main.async { KoreanKeyTap.goToKorean() }
            }
            HardwareKeys.katakanaDown = down
        }, nil)
        IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
        let status = IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone))
        // Exclusive access: a keyboard another program holds (Karabiner-Elements seizes the ones it
        // remaps) didn't open, but the others did. Only "not permitted" means waiting.
        guard status == kIOReturnSuccess || status == kIOReturnExclusiveAccess else {
            // Not allowed yet (Input Monitoring): try again until it is, without a restart.
            IOHIDManagerUnscheduleFromRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
            let access = IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)
            let state = "open \(status), Input Monitoring \(access.rawValue) (0 granted, 1 denied, 2 unknown), Accessibility \(AXIsProcessTrusted())"
            if state != lastState {
                DiagnosticLog.note("hardware keys: waiting: \(state)")
                lastState = state
            }
            waiting = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { start() }
            return
        }
        DiagnosticLog.note("hardware keys: on for \(keyboards.count) JIS keyboard(s)\(status == kIOReturnSuccess ? "" : ", some held by another program")")
        self.manager = manager
    }

    private static var waiting = false

    /// The keyboards macOS has configured as JIS: /Library/Preferences/com.apple.keyboardtype lists
    /// each keyboard ("product-vendor-country") with its keyboard type.
    private static func jisKeyboards() -> [(vendor: Int, product: Int)] {
        let types = (UserDefaults(suiteName: "/Library/Preferences/com.apple.keyboardtype")?
            .dictionary(forKey: "keyboardtype") as? [String: Int]) ?? [:]
        var keyboards: [(vendor: Int, product: Int)] = []
        for (device, type) in types where KBGetLayoutType(Int16(type)) == OSType(kKeyboardJIS) {
            let parts = device.split(separator: "-").compactMap { Int($0) }
            if parts.count >= 2, !keyboards.contains(where: { $0.vendor == parts[1] && $0.product == parts[0] }) {
                keyboards.append((vendor: parts[1], product: parts[0]))
            }
        }
        return keyboards
    }
    private static var lastState = ""

    /// Whether カタカナ is held (for a key down), or was just let go (for its key up: the release
    /// may be seen first).
    static func katakanaHeld(release: Bool) -> Bool {
        katakanaDown || (release && Date().timeIntervalSince(katakanaReleased) < 0.3)
    }
}
#endif
