// Registers an input method bundle with Text Input Sources, so it shows up in
// System Settings → Keyboard → Input Sources without logging out.
// Usage: swift register.swift <path-to-app>
import Carbon

guard CommandLine.arguments.count > 1 else { fatalError("usage: register.swift <app>") }
let url = URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL
let status = TISRegisterInputSource(url)
print(status == noErr ? "Registered." : "TISRegisterInputSource failed: \(status)")
exit(status == noErr ? 0 : 1)
