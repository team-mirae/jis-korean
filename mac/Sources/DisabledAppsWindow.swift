import Cocoa

#if PREDICT
/// "Manage Disabled Apps…": every app excluded from learning or predictions, with its canonical
/// location fetched when the window loads. An app that's no longer installed stays listed, with an
/// ⓘ that says so; it's only removed when the user asks (and confirms).
final class DisabledAppsWindow: NSObject {
    static let shared = DisabledAppsWindow()

    private lazy var panel: NSPanel = {
        let panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 620, height: 360),
                            styleMask: [.titled, .closable, .resizable, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.title = "JIS Korean: Disabled Apps"
        panel.level = .floating
        panel.hidesOnDeactivate = false
        panel.becomesKeyOnlyIfNeeded = true
        panel.isReleasedWhenClosed = false
        panel.minSize = NSSize(width: 480, height: 200)
        return panel
    }()

    private var popover: NSPopover?

    func show() {
        reload()
        panel.center()
        panel.orderFrontRegardless()
    }

    /// Builds the list, fetching each app's location now.
    private func reload() {
        let rows = NSStackView()
        rows.orientation = .vertical
        rows.alignment = .leading
        rows.spacing = 10
        rows.edgeInsets = NSEdgeInsets(top: 14, left: 16, bottom: 14, right: 16)
        let apps = DisabledApps.all()
        if apps.isEmpty {
            rows.addArrangedSubview(label("No apps are disabled. Use the input menu's “Disable … in this app” items to add one.", secondary: true))
        }
        for app in apps {
            let path = DisabledApps.location(app.bundleID)
            if let path {
                DisabledApps.refresh(app.bundleID, path: path, name: DisabledApps.displayName(app.bundleID))
            }
            rows.addArrangedSubview(row(app, path: path))
        }
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        let flipped = FlippedView()
        flipped.addSubview(rows)
        rows.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            rows.topAnchor.constraint(equalTo: flipped.topAnchor),
            rows.leadingAnchor.constraint(equalTo: flipped.leadingAnchor),
            rows.trailingAnchor.constraint(equalTo: flipped.trailingAnchor),
            rows.bottomAnchor.constraint(equalTo: flipped.bottomAnchor),
        ])
        scroll.documentView = flipped
        flipped.translatesAutoresizingMaskIntoConstraints = false
        flipped.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor).isActive = true
        panel.contentView = scroll
    }

    private func row(_ app: DisabledApp, path: String?) -> NSView {
        let icon = NSImageView(image: path.map { NSWorkspace.shared.icon(forFile: $0) }
            ?? NSImage(systemSymbolName: "app.dashed", accessibilityDescription: "App not found") ?? NSImage())
        icon.translatesAutoresizingMaskIntoConstraints = false
        icon.widthAnchor.constraint(equalToConstant: 32).isActive = true
        icon.heightAnchor.constraint(equalToConstant: 32).isActive = true

        let name = label(app.name, secondary: false)
        name.font = .boldSystemFont(ofSize: NSFont.systemFontSize)
        let location = label(path ?? app.path, secondary: true)
        location.lineBreakMode = .byTruncatingMiddle
        location.toolTip = path ?? app.path
        var titleViews: [NSView] = [name]
        if path == nil {
            // Uninstalled: kept until the user removes it; the ⓘ says so.
            let message = "\(app.name) was deleted: it's no longer installed on this Mac (last seen at \(app.path.isEmpty ? app.bundleID : app.path)). The entry stays until you remove it."
            let info = NSButton(image: NSImage(systemSymbolName: "info.circle", accessibilityDescription: "Deleted app") ?? NSImage(),
                                target: self, action: #selector(showInfo(_:)))
            info.isBordered = false
            info.toolTip = message
            info.identifier = NSUserInterfaceItemIdentifier(message)
            titleViews.append(info)
            location.stringValue = "Not installed · last seen at \(app.path.isEmpty ? "unknown location" : app.path)"
        }
        let title = NSStackView(views: titleViews)
        title.spacing = 4
        let text = NSStackView(views: [title, location])
        text.orientation = .vertical
        text.alignment = .leading
        text.spacing = 2
        text.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        let learning = NSButton(checkboxWithTitle: "Learning", target: self, action: #selector(toggleLearning(_:)))
        learning.state = app.noLearning ? .off : .on
        learning.identifier = NSUserInterfaceItemIdentifier(app.bundleID)
        let predictions = NSButton(checkboxWithTitle: "Predictions", target: self, action: #selector(togglePredictions(_:)))
        predictions.state = DefaultOffApps.predicting(app.bundleID) ? .on : .off
        if let reason = DefaultOffApps.reason(app.bundleID) {
            predictions.toolTip = "Off by default here: \(reason.notice)."
        }
        predictions.identifier = NSUserInterfaceItemIdentifier(app.bundleID)
        let remove = NSButton(title: "Remove…", target: self, action: #selector(remove(_:)))
        remove.identifier = NSUserInterfaceItemIdentifier(app.bundleID)

        let row = NSStackView(views: [icon, text, NSView(), learning, predictions, remove])
        row.spacing = 12
        row.alignment = .centerY
        row.translatesAutoresizingMaskIntoConstraints = false
        row.widthAnchor.constraint(greaterThanOrEqualToConstant: 560).isActive = true
        return row
    }

    private func label(_ s: String, secondary: Bool) -> NSTextField {
        let field = NSTextField(labelWithString: s)
        if secondary {
            field.textColor = .secondaryLabelColor
            field.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        }
        return field
    }

    @objc private func showInfo(_ sender: NSButton) {
        popover?.close()
        let controller = NSViewController()
        let text = NSTextField(wrappingLabelWithString: sender.identifier?.rawValue ?? "")
        text.preferredMaxLayoutWidth = 280
        let box = NSStackView(views: [text])
        box.edgeInsets = NSEdgeInsets(top: 10, left: 12, bottom: 10, right: 12)
        controller.view = box
        let popover = NSPopover()
        popover.contentViewController = controller
        popover.behavior = .transient
        popover.show(relativeTo: sender.bounds, of: sender, preferredEdge: .maxY)
        self.popover = popover
    }

    @objc private func toggleLearning(_ sender: NSButton) {
        guard let id = sender.identifier?.rawValue else { return }
        DisabledApps.update(id) { $0.noLearning = sender.state == .off }
        reload()
    }

    @objc private func togglePredictions(_ sender: NSButton) {
        guard let id = sender.identifier?.rawValue else { return }
        let defaultOff = DefaultOffApps.reason(id) != nil
        DisabledApps.update(id) {
            $0.noPredictions = sender.state == .off && !defaultOff
            $0.allowPredictions = sender.state == .on && defaultOff
        }
        reload()
    }

    @objc private func remove(_ sender: NSButton) {
        guard let id = sender.identifier?.rawValue, let app = DisabledApps.entry(id) else { return }
        let alert = NSAlert()
        alert.messageText = "Remove \(app.name) from the list?"
        alert.informativeText = "Learning and predictions will be on in it again."
        alert.addButton(withTitle: "Remove")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: panel) { [weak self] response in
            if response == .alertFirstButtonReturn {
                DisabledApps.remove(id)
                self?.reload()
            }
        }
    }
}

/// A document view that lays out from the top.
private final class FlippedView: NSView {
    override var isFlipped: Bool { true }
}
#endif
