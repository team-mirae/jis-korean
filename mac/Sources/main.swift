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

#if PREDICT
/// The predictive layer (spec §10), parsed once and shared. Nil if its files are missing.
let sharedPredictor: OpaquePointer? = {
    func data(_ name: String, _ ext: String) -> Data? {
        Bundle.main.url(forResource: name, withExtension: ext, subdirectory: "predict").flatMap { try? Data(contentsOf: $0) }
    }
    guard let model = data("ranker", "bin"), let english = data("common-en", "txt"), let korean = data("common-ko", "txt")
    else { return nil }
    let nouns = data("english-nouns", "txt") ?? Data()
    return model.withUnsafeBytes { m in
        english.withUnsafeBytes { e in
            korean.withUnsafeBytes { k in
                nouns.withUnsafeBytes { n in
                    jk_predictor_new(
                        m.bindMemory(to: UInt8.self).baseAddress, m.count,
                        e.bindMemory(to: UInt8.self).baseAddress, e.count,
                        k.bindMemory(to: UInt8.self).baseAddress, k.count,
                        n.bindMemory(to: UInt8.self).baseAddress, n.count)
                }
            }
        }
    }
}()
#endif

let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)
_ = server
NSApplication.shared.run()
