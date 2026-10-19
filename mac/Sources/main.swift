import Cocoa
import InputMethodKit

// Input methods are background apps: IMKServer accepts client connections, and the system creates
// one JKInputController per text input client.
let connectionName = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String
    ?? "me.sjun.inputmethod.JISKorean_Connection"
/// The hanja dictionary, parsed once and shared by every controller's engine.
let sharedDictionary: OpaquePointer? = {
    guard let url = Bundle.main.url(forResource: "hanja", withExtension: "tsv"),
          let data = try? Data(contentsOf: url)
    else { return nil }
    return data.withUnsafeBytes { bytes in
        jk_dict_new(bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
    }
}()

let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)
_ = server
NSApplication.shared.run()
