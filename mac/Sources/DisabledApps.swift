import Cocoa

#if PREDICT
/// Apps the user excluded from learning or from predictions (spec §10.11): kept in the preferences
/// (`DisabledApps`) as bundle identifiers, with the app's name and last known location so an
/// uninstalled app can still be shown.
struct DisabledApp {
    let bundleID: String
    var name: String
    var path: String
    /// Choices made in this app aren't recorded (what was learned still applies).
    var noLearning: Bool
    /// No predictions in this app: the deterministic engine only.
    var noPredictions: Bool
    /// Predictions turned on by the user in an app where they start off (DefaultOffApps).
    var allowPredictions: Bool

    fileprivate var dictionary: [String: Any] {
        ["bundleID": bundleID, "name": name, "path": path, "noLearning": noLearning, "noPredictions": noPredictions,
         "allowPredictions": allowPredictions]
    }

    fileprivate init?(_ d: [String: Any]) {
        guard let id = d["bundleID"] as? String else { return nil }
        bundleID = id
        name = d["name"] as? String ?? id
        path = d["path"] as? String ?? ""
        noLearning = d["noLearning"] as? Bool ?? false
        noPredictions = d["noPredictions"] as? Bool ?? false
        allowPredictions = d["allowPredictions"] as? Bool ?? false
    }

    init(bundleID: String, name: String, path: String) {
        self.bundleID = bundleID
        self.name = name
        self.path = path
        noLearning = false
        noPredictions = false
        allowPredictions = false
    }
}

enum DisabledApps {
    private static let key = "DisabledApps"

    static func all() -> [DisabledApp] {
        (UserDefaults.standard.array(forKey: key) as? [[String: Any]] ?? []).compactMap(DisabledApp.init)
    }

    static func entry(_ bundleID: String?) -> DisabledApp? {
        guard let bundleID else { return nil }
        return all().first { $0.bundleID == bundleID }
    }

    private static func save(_ apps: [DisabledApp]) {
        UserDefaults.standard.set(apps.map(\.dictionary), forKey: key)
    }

    /// Changes one app's settings (adding it if new). An app with nothing set is dropped: the user
    /// turned everything back to how it starts themselves.
    static func update(_ bundleID: String, _ change: (inout DisabledApp) -> Void) {
        var apps = all()
        var app = apps.first { $0.bundleID == bundleID } ?? DisabledApp(bundleID: bundleID, name: displayName(bundleID), path: location(bundleID) ?? "")
        change(&app)
        apps.removeAll { $0.bundleID == bundleID }
        if app.noLearning || app.noPredictions || app.allowPredictions {
            apps.append(app)
        }
        save(apps)
    }

    /// Takes an app off the list (only ever on the user's say-so).
    static func remove(_ bundleID: String) {
        save(all().filter { $0.bundleID != bundleID })
    }

    /// Records where an app is now (its canonical full path) and its current name.
    static func refresh(_ bundleID: String, path: String, name: String) {
        var apps = all()
        guard let i = apps.firstIndex(where: { $0.bundleID == bundleID }) else { return }
        apps[i].path = path
        apps[i].name = name
        save(apps)
    }

    /// The app's canonical full path (symlinks resolved), or nil when it isn't installed.
    static func location(_ bundleID: String) -> String? {
        NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID)?.resolvingSymlinksInPath().standardizedFileURL.path
    }

    /// The app's name as Finder shows it, or the bundle identifier.
    static func displayName(_ bundleID: String) -> String {
        if let running = NSRunningApplication.runningApplications(withBundleIdentifier: bundleID).first?.localizedName {
            return running
        }
        if let path = location(bundleID) {
            return FileManager.default.displayName(atPath: path).replacingOccurrences(of: ".app", with: "")
        }
        return bundleID
    }
}
#endif
