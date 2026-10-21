import IOKit
import IOKit.hidsystem

/// The Caps Lock state, which JIS Korean turns off when plain Caps Lock is its switch key
/// (spec §8.2.1): the press reaches the input method only after macOS has toggled the lock, so the
/// lock is set back instead of the press being swallowed.
enum CapsLock {
    static func turnOff() {
        let service = IOServiceGetMatchingService(kIOMainPortDefault, IOServiceMatching(kIOHIDSystemClass))
        guard service != 0 else { return }
        defer { IOObjectRelease(service) }
        var connect: io_connect_t = 0
        guard IOServiceOpen(service, mach_task_self_, UInt32(kIOHIDParamConnectType), &connect) == KERN_SUCCESS else {
            DiagnosticLog.note("Caps Lock: couldn't open the HID system to turn it off")
            return
        }
        defer { IOServiceClose(connect) }
        IOHIDSetModifierLockState(connect, Int32(kIOHIDCapsLockState), false)
    }
}
