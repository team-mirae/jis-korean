import Foundation
import IOKit
import IOKit.hidsystem

/// The Caps Lock state. Plain Caps Lock is JIS Korean's switch key (spec §8.2.1), but its press
/// reaches the input method only after macOS has toggled the lock, so the lock is set back to what
/// it was instead of the press being swallowed.
enum CapsLock {
    /// The state JIS Korean last set, and when: the change can come back as a Caps Lock event of
    /// its own, which isn't a press.
    private static var pending: (on: Bool, at: Date)?

    static func set(_ on: Bool) {
        let service = IOServiceGetMatchingService(kIOMainPortDefault, IOServiceMatching(kIOHIDSystemClass))
        guard service != 0 else { return }
        defer { IOObjectRelease(service) }
        var connect: io_connect_t = 0
        guard IOServiceOpen(service, mach_task_self_, UInt32(kIOHIDParamConnectType), &connect) == KERN_SUCCESS else {
            DiagnosticLog.note("Caps Lock: couldn't open the HID system to set it")
            return
        }
        defer { IOServiceClose(connect) }
        pending = (on, Date())
        IOHIDSetModifierLockState(connect, Int32(kIOHIDCapsLockState), on)
    }

    /// Whether a Caps Lock event with the lock `on` is the echo of JIS Korean's own last change
    /// (within half a second). Each change has at most one echo.
    static func isEcho(_ on: Bool) -> Bool {
        guard let pending, pending.on == on, Date().timeIntervalSince(pending.at) < 0.5 else { return false }
        self.pending = nil
        return true
    }
}
