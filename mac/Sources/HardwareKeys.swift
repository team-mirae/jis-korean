import Foundation
import IOKit.hid

#if ENGLISH
/// The keyboard read at the hardware level (USB HID usages), for the one key macOS loses: the Windows
/// JIS カタカナ key (usage 0x88), which macOS turns into かな (spec §8.2.1). Only whether it's held is
/// kept. Needs Input Monitoring permission, which it asks for.
enum HardwareKeys {
    private static var manager: IOHIDManager?
    fileprivate static var katakanaDown = false
    fileprivate static var katakanaReleased = Date.distantPast

    static func start() {
        guard manager == nil else { return }
        if IOHIDCheckAccess(kIOHIDRequestTypeListenEvent) != kIOHIDAccessTypeGranted {
            _ = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)
        }
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
        let keyboards = [kIOHIDDeviceUsagePageKey: kHIDPage_GenericDesktop, kIOHIDDeviceUsageKey: kHIDUsage_GD_Keyboard]
        IOHIDManagerSetDeviceMatching(manager, keyboards as CFDictionary)
        IOHIDManagerRegisterInputValueCallback(manager, { _, _, _, value in
            let element = IOHIDValueGetElement(value)
            guard IOHIDElementGetUsagePage(element) == UInt32(kHIDPage_KeyboardOrKeypad),
                  IOHIDElementGetUsage(element) == UInt32(kHIDUsage_KeyboardInternational2)  // 0x88, カタカナ
            else { return }
            let down = IOHIDValueGetIntegerValue(value) != 0
            if !down { HardwareKeys.katakanaReleased = Date() }
            HardwareKeys.katakanaDown = down
        }, nil)
        IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
        let status = IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone))
        DiagnosticLog.note("hardware keys: \(status == kIOReturnSuccess ? "on" : "couldn't open (\(status)); Input Monitoring?")")
        self.manager = manager
    }

    /// Whether カタカナ is held (for a key down), or was just let go (for its key up: the release
    /// may be seen first).
    static func katakanaHeld(release: Bool) -> Bool {
        katakanaDown || (release && Date().timeIntervalSince(katakanaReleased) < 0.3)
    }
}
#endif
