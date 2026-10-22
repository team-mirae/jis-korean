import Cocoa
import CryptoKit

#if PREDICT
/// The training contract (docs/training-contract.md, spec §10.12) on macOS: a training file at
/// `~/Library/Application Support/JIS Korean/training.json` is read when JIS Korean activates and
/// the file has changed since last time. Its weights become the `Weight.<Scenario>` preferences
/// (set by hand), its words are kept in the preferences and added to the shared predictor, and its
/// examples are learned from (turning "Learn from My Choices" on, since only then do they count).
enum Training {
    static let folder = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("JIS Korean", isDirectory: true)
    static let file = folder.appendingPathComponent("training.json")

    private static let appliedDateKey = "TrainingFileDate"
    /// The SHA-256 of the file last applied: a new date with the same content (touched, saved
    /// unchanged, rewritten by a sync tool) applies nothing, so its examples aren't learned twice.
    private static let appliedHashKey = "TrainingFileHash"
    private static let koreanWordsKey = "TrainingWords.Korean"
    private static let englishWordsKey = "TrainingWords.English"
    private static let weightKeys = [
        "korean": "Korean", "english": "English", "implied": "Implied", "mixed": "Mixed", "two-set": "TwoSet",
        "keep-finals": "KeepFinals", "hanja": "Hanja", "implied-l": "ImpliedL",
    ]
    private static let confidenceCount = ["tentative": 10, "likely": 20, "confirmed": 50]

    /// The shared predictor with the training words, built once per change of words.
    private static var personal: (words: String, predictor: OpaquePointer?)?

    /// The predictor engines use: the shared one, plus the user's training words if any.
    static func predictor() -> OpaquePointer? {
        let defaults = UserDefaults.standard
        let korean = defaults.string(forKey: koreanWordsKey) ?? ""
        let english = defaults.string(forKey: englishWordsKey) ?? ""
        if korean.isEmpty && english.isEmpty {
            return sharedPredictor
        }
        let words = korean + "\u{0}" + english
        if let personal, personal.words == words {
            return personal.predictor
        }
        let k = Array(korean.utf8), e = Array(english.utf8)
        let built = jk_predictor_with_words(sharedPredictor, k, k.count, e, e.count)
        if let old = personal?.predictor {
            jk_predictor_free(old) // engines keep their own reference
        }
        personal = (words, built)
        return built ?? sharedPredictor
    }

    /// Applies the training file if it changed since it was last applied (or forgets its words if
    /// it's gone). Examples are learned into `engine`; the caller saves what was learned.
    static func applyIfChanged(to engine: OpaquePointer?) {
        let defaults = UserDefaults.standard
        let date = (try? FileManager.default.attributesOfItem(atPath: file.path)[.modificationDate]) as? Date
        guard let date else {
            if defaults.object(forKey: appliedDateKey) != nil {
                defaults.removeObject(forKey: appliedDateKey)
                defaults.removeObject(forKey: appliedHashKey)
                defaults.removeObject(forKey: koreanWordsKey)
                defaults.removeObject(forKey: englishWordsKey)
            }
            return
        }
        if let applied = defaults.object(forKey: appliedDateKey) as? Date, applied == date {
            return
        }
        defaults.set(date, forKey: appliedDateKey)
        guard let data = try? Data(contentsOf: file) else { return }
        let hash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        if defaults.string(forKey: appliedHashKey) == hash {
            return
        }
        defaults.set(hash, forKey: appliedHashKey)
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              json["format"] as? String == "jis-korean-training", json["version"] as? Int == 1
        else {
            NSLog("JIS Korean: %@ isn't a version 1 training file; ignored", file.path)
            return
        }
        for (name, value) in json["weights"] as? [String: Any] ?? [:] {
            if let key = weightKeys[name], let number = value as? Double, (0...10).contains(number) {
                defaults.set(number, forKey: "Weight.\(key)")
            }
        }
        let vocabulary = json["vocabulary"] as? [String: Any] ?? [:]
        func lines(_ list: Any?, _ pattern: String) -> String {
            (list as? [[String: Any]] ?? []).compactMap { entry in
                guard let word = entry["word"] as? String, word.range(of: pattern, options: .regularExpression) != nil
                else { return nil }
                let count = entry["count"] as? Int ?? (entry["confidence"] as? String).flatMap { confidenceCount[$0] } ?? 1000
                return "\(word)\t\(count)"
            }.prefix(5000).joined(separator: "\n")
        }
        defaults.set(lines(vocabulary["korean"], "^[가-힣]+$"), forKey: koreanWordsKey)
        defaults.set(lines(vocabulary["english"], "^[A-Za-z]+$"), forKey: englishWordsKey)
        let examples = (json["examples"] as? [[String: Any]] ?? []).prefix(2000)
        if !examples.isEmpty {
            defaults.set(true, forKey: "LearnFromChoices")
            jk_set_predictor(engine, predictor())
            for example in examples {
                guard let text = example["text"] as? String else { continue }
                let t = Array(text.utf8), k = Array((example["keys"] as? String ?? "").utf8)
                for _ in 0..<min(max(example["count"] as? Int ?? 1, 1), 20) {
                    _ = jk_learn_example(engine, t, t.count, k, k.count)
                }
            }
        }
        NSLog("JIS Korean: applied %@", file.path)
    }

    /// Opens the folder the training file goes in (creating it), in Finder.
    static func openFolder() {
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(folder)
    }
}
#endif
