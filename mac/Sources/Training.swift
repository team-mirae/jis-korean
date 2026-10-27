import Cocoa
import CryptoKit

#if PREDICT
/// The training contract (docs/training-contract.md, spec §10.12) on macOS: a training file at
/// `~/Library/Application Support/JIS Korean/training.json` is read when JIS Korean activates and
/// the file has changed since last time. Its weights become the `Weight.<Scenario>` preferences
/// (set by hand), its words and avoided words are kept in the preferences and added to the shared
/// predictor, and its examples are learned from (turning "Learn from My Choices" on, since only then do they count).
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
    private static let koreanAvoidKey = "TrainingAvoid.Korean"
    private static let englishAvoidKey = "TrainingAvoid.English"
    private static let weightKeys = [
        "korean": "Korean", "english": "English", "implied": "Implied", "mixed": "Mixed", "two-set": "TwoSet",
        "keep-finals": "KeepFinals", "hanja": "Hanja", "implied-l": "ImpliedL",
    ]
    private static let confidenceCount = ["tentative": 10, "likely": 20, "confirmed": 50]

    /// The shared predictor with the training words, built once per change of words.
    private static var personal: (words: String, predictor: OpaquePointer?)?

    /// The predictor engines use: the shared one, plus the user's training words and avoided words
    /// if any.
    static func predictor() -> OpaquePointer? {
        let defaults = UserDefaults.standard
        let korean = defaults.string(forKey: koreanWordsKey) ?? ""
        let english = defaults.string(forKey: englishWordsKey) ?? ""
        let avoidKorean = defaults.string(forKey: koreanAvoidKey) ?? ""
        let avoidEnglish = defaults.string(forKey: englishAvoidKey) ?? ""
        if korean.isEmpty && english.isEmpty && avoidKorean.isEmpty && avoidEnglish.isEmpty {
            return sharedPredictor
        }
        let words = [korean, english, avoidKorean, avoidEnglish].joined(separator: "\u{0}")
        if let personal, personal.words == words {
            return personal.predictor
        }
        let k = Array(korean.utf8), e = Array(english.utf8)
        let withWords = jk_predictor_with_words(sharedPredictor, k, k.count, e, e.count)
        let ak = Array(avoidKorean.utf8), ae = Array(avoidEnglish.utf8)
        // Only when there's something to avoid: each predictor is a full copy (about 5 MB).
        let avoids = !(avoidKorean.isEmpty && avoidEnglish.isEmpty)
        let built = avoids ? withWords.flatMap { jk_predictor_with_avoided($0, ak, ak.count, ae, ae.count) } ?? withWords : withWords
        if built != withWords, let withWords {
            jk_predictor_free(withWords)
        }
        if let old = personal?.predictor {
            jk_predictor_free(old) // engines keep their own reference
        }
        personal = (words, built)
        return built ?? sharedPredictor
    }

    /// Whether a training file's examples are being learned (in the background).
    private static var learning = false
    private static let queue = DispatchQueue(label: "me.sjun.inputmethod.JISKorean.training", qos: .utility)

    /// Applies the training file if it changed since it was last applied (or forgets its words if
    /// it's gone). Its weights and words apply at once; its examples are learned in the background,
    /// on an engine of their own, so activating a field never waits on them (#13). What they teach
    /// is stored as what was learned (`Learned`) and applied to the field being typed in when done;
    /// other fields load it when they're activated. The file counts as applied only then.
    static func applyIfChanged() {
        guard !learning else { return }
        let defaults = UserDefaults.standard
        let date = (try? FileManager.default.attributesOfItem(atPath: file.path)[.modificationDate]) as? Date
        guard let date else {
            if defaults.object(forKey: appliedDateKey) != nil {
                defaults.removeObject(forKey: appliedDateKey)
                defaults.removeObject(forKey: appliedHashKey)
                defaults.removeObject(forKey: koreanWordsKey)
                defaults.removeObject(forKey: englishWordsKey)
                defaults.removeObject(forKey: koreanAvoidKey)
                defaults.removeObject(forKey: englishAvoidKey)
            }
            return
        }
        if let applied = defaults.object(forKey: appliedDateKey) as? Date, applied == date {
            return
        }
        guard let data = try? Data(contentsOf: file) else { return }
        let hash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        let markApplied = {
            defaults.set(date, forKey: appliedDateKey)
            defaults.set(hash, forKey: appliedHashKey)
        }
        if defaults.string(forKey: appliedHashKey) == hash {
            defaults.set(date, forKey: appliedDateKey)
            return
        }
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              json["format"] as? String == "jis-korean-training", json["version"] as? Int == 1
        else {
            NSLog("JIS Korean: %@ isn't a version 1 training file; ignored", file.path)
            markApplied()
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
        func avoided(_ language: String, _ pattern: String) -> String {
            (json["avoid"] as? [[String: Any]] ?? []).compactMap { entry in
                guard entry["language"] as? String == language, let word = entry["word"] as? String,
                      word.range(of: pattern, options: .regularExpression) != nil
                else { return nil }
                return entry["strength"] as? String == "strong" ? "\(word)\tstrong" : word
            }.prefix(5000).joined(separator: "\n")
        }
        defaults.set(avoided("korean", "^[가-힣]+$"), forKey: koreanAvoidKey)
        defaults.set(avoided("english", "^[A-Za-z]+$"), forKey: englishAvoidKey)
        let examples: [(text: [UInt8], keys: [UInt8], count: Int)] = (json["examples"] as? [[String: Any]] ?? []).prefix(2000).compactMap { example in
            guard let text = example["text"] as? String else { return nil }
            return (Array(text.utf8), Array((example["keys"] as? String ?? "").utf8), min(max(example["count"] as? Int ?? 1, 1), 20))
        }
        guard !examples.isEmpty else {
            markApplied()
            NSLog("JIS Korean: applied %@", file.path)
            return
        }
        defaults.set(true, forKey: "LearnFromChoices")
        // An engine of their own, set up on this thread as the fields' engines are (the predictor
        // with the training words, the weights, what was learned so far); the predictor is shared
        // read-only, so it's safe to use from the background.
        guard let scratch = jk_new() else { return }
        jk_set_predictor(scratch, predictor())
        for (id, name) in weightNames.enumerated() {
            if let value = defaults.object(forKey: "Weight.\(name)") as? Double {
                jk_set_prediction_weight(scratch, UInt32(id), value)
            }
        }
        if let learned = defaults.string(forKey: learnedDefault) {
            let bytes = Array(learned.utf8)
            _ = jk_learner_import(scratch, bytes, bytes.count)
        }
        learning = true
        let handle = UInt(bitPattern: scratch)
        queue.async {
            guard let scratch = OpaquePointer(bitPattern: handle) else { return }
            for example in examples {
                for _ in 0..<example.count {
                    _ = jk_learn_example(scratch, example.text, example.text.count, example.keys, example.keys.count)
                }
            }
            let length = jk_learner_export(scratch)
            let learned = jk_learner_ptr(scratch).map { String(decoding: UnsafeBufferPointer(start: $0, count: length), as: UTF8.self) }
            jk_free(scratch)
            DispatchQueue.main.async {
                learning = false
                if let learned, !learned.isEmpty {
                    UserDefaults.standard.set(learned, forKey: learnedDefault)
                }
                markApplied()
                NSLog("JIS Korean: applied %@ (%d examples)", file.path, examples.count)
                JKInputController.trainingApplied()
            }
        }
    }

    /// Opens the folder the training file goes in (creating it), in Finder.
    static func openFolder() {
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(folder)
    }
}
#endif
