import Cocoa

/// The suggestion list under the marked text: predictions while typing, hanja candidates after F7.
/// Modeled on the macOS Japanese input method's live-conversion list: a rounded Liquid Glass panel,
/// one row per suggestion with hairline separators, and a slim scroll thumb when it overflows. The
/// English modes' letter variants (spec §3.6.3) are single letters, so they run in one row instead,
/// each with the digit that picks it.
final class SuggestionWindow {
    static let shared = SuggestionWindow()

    /// Called with a row index when a row is clicked.
    var onPick: ((Int) -> Void)?

    private let panel: NSPanel
    private let list = SuggestionListView()

    private init() {
        panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.level = .popUpMenu
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.hidesOnDeactivate = false
        panel.becomesKeyOnlyIfNeeded = true
        // Readings of what's being typed: never in screen sharing, recordings or screenshots (as on
        // Windows).
        panel.sharingType = .none

        let glass: NSView
        if #available(macOS 26.0, *) {
            let view = NSGlassEffectView()
            view.cornerRadius = SuggestionListView.cornerRadius
            view.contentView = list
            glass = view
        } else {
            let view = NSVisualEffectView()
            view.material = .menu
            view.blendingMode = .behindWindow
            view.state = .active
            view.wantsLayer = true
            view.layer?.cornerRadius = SuggestionListView.cornerRadius
            view.layer?.masksToBounds = true
            list.frame = view.bounds
            list.autoresizingMask = [.width, .height]
            view.addSubview(list)
            glass = view
        }
        // The hairline rim around the glass.
        glass.wantsLayer = true
        glass.layer?.cornerRadius = SuggestionListView.cornerRadius
        glass.layer?.borderWidth = 0.5
        glass.layer?.borderColor = NSColor.white.withAlphaComponent(0.25).cgColor
        panel.contentView = glass
        list.onPick = { [weak self] index in self?.onPick?(index) }
    }

    /// Shows `items` under the caret rectangle `caret` (screen coordinates); `selected` is the
    /// highlighted item, or nil while nothing is chosen yet. `horizontal`: one row of single letters.
    func show(_ items: [String], selected: Int?, below caret: NSRect, horizontal: Bool = false) {
        guard !items.isEmpty else { return hide() }
        list.horizontal = horizontal
        list.update(items: items, selected: selected)
        let size = list.preferredSize
        var origin = NSPoint(x: caret.minX - SuggestionListView.textInset, y: caret.minY - 6 - size.height)
        if let screen = NSScreen.screens.first(where: { $0.frame.contains(caret.origin) }) ?? NSScreen.main {
            let visible = screen.visibleFrame
            if origin.y < visible.minY {
                origin.y = caret.maxY + 6  // no room below: show above the text
            }
            origin.x = min(max(origin.x, visible.minX + 4), visible.maxX - size.width - 4)
        }
        panel.setFrame(NSRect(origin: origin, size: size), display: true)
        panel.orderFrontRegardless()
    }

    func hide() {
        panel.orderOut(nil)
    }
}

/// The rows, drawn directly so they match the system list exactly.
private final class SuggestionListView: NSView {
    static let cornerRadius: CGFloat = 13
    static let textInset: CGFloat = 12
    private static let rowHeight: CGFloat = 27
    private static let visibleRows = 5
    private static let font = NSFont.systemFont(ofSize: 15)

    var onPick: ((Int) -> Void)?
    /// One row of cells, each a letter over the digit that picks it (1–9, 0 for the tenth, none
    /// after), instead of a column of rows.
    var horizontal = false
    private static let cellWidth: CGFloat = 34
    private static let cellHeight: CGFloat = 46
    private static let letterFont = NSFont.systemFont(ofSize: 19)
    private static let digitFont = NSFont.monospacedDigitSystemFont(ofSize: 10, weight: .medium)
    private var items: [String] = []
    private var selected: Int?
    private var firstVisible = 0

    override var isFlipped: Bool { true }

    var preferredSize: NSSize {
        if horizontal {
            let widest = items.map { ($0 as NSString).size(withAttributes: [.font: Self.letterFont]).width }.max() ?? 0
            let cell = max(Self.cellWidth, ceil(widest) + 12)
            return NSSize(width: CGFloat(items.count) * cell + 10, height: Self.cellHeight + 10)
        }
        let widest = items.map { ($0 as NSString).size(withAttributes: [.font: Self.font]).width }.max() ?? 0
        let scroll: CGFloat = items.count > Self.visibleRows ? 14 : 0
        let width = max(180, ceil(widest) + Self.textInset * 2 + 8 + scroll)
        let rows = min(items.count, Self.visibleRows)
        return NSSize(width: width, height: CGFloat(rows) * Self.rowHeight + 10)
    }

    func update(items: [String], selected: Int?) {
        self.items = items
        self.selected = selected
        // Keep the selected row in view.
        if let selected {
            if selected < firstVisible { firstVisible = selected }
            if selected >= firstVisible + Self.visibleRows { firstVisible = selected - Self.visibleRows + 1 }
        }
        firstVisible = min(firstVisible, max(0, items.count - Self.visibleRows))
        needsDisplay = true
    }

    private var cellWidth: CGFloat {
        horizontal && !items.isEmpty ? (bounds.width - 10) / CGFloat(items.count) : Self.cellWidth
    }

    private func drawHorizontal() {
        let width = cellWidth
        for (index, item) in items.enumerated() {
            let cell = NSRect(x: 5 + CGFloat(index) * width, y: 5, width: width, height: Self.cellHeight)
            let isSelected = index == selected
            if isSelected {
                NSColor.controlAccentColor.withAlphaComponent(0.85).setFill()
                NSBezierPath(roundedRect: cell.insetBy(dx: 1, dy: 0), xRadius: 8, yRadius: 8).fill()
            }
            let color = isSelected ? NSColor.white : NSColor.labelColor
            let letter = item as NSString
            let letterAttributes: [NSAttributedString.Key: Any] = [.font: Self.letterFont, .foregroundColor: color]
            let size = letter.size(withAttributes: letterAttributes)
            letter.draw(at: NSPoint(x: cell.midX - size.width / 2, y: cell.minY + 5), withAttributes: letterAttributes)
            // 1–9, then 0 for the tenth; later ones have no digit.
            if index < 10 {
                let digit = String((index + 1) % 10) as NSString
                let digitAttributes: [NSAttributedString.Key: Any] = [
                    .font: Self.digitFont,
                    .foregroundColor: isSelected ? NSColor.white.withAlphaComponent(0.85) : NSColor.secondaryLabelColor,
                ]
                let digitSize = digit.size(withAttributes: digitAttributes)
                digit.draw(at: NSPoint(x: cell.midX - digitSize.width / 2, y: cell.maxY - digitSize.height - 4),
                           withAttributes: digitAttributes)
            }
        }
    }

    override func draw(_ dirtyRect: NSRect) {
        if horizontal { return drawHorizontal() }
        let rows = min(items.count, Self.visibleRows)
        let overflow = items.count > Self.visibleRows
        let rightEdge = bounds.width - (overflow ? 14 : 0)
        for row in 0..<rows {
            let index = firstVisible + row
            let top = 5 + CGFloat(row) * Self.rowHeight
            let rowRect = NSRect(x: 5, y: top, width: rightEdge - 10, height: Self.rowHeight)
            let isSelected = index == selected
            if isSelected {
                NSColor.controlAccentColor.withAlphaComponent(0.85).setFill()
                NSBezierPath(roundedRect: rowRect.insetBy(dx: 0, dy: 1), xRadius: 7, yRadius: 7).fill()
            }
            let text = items[index] as NSString
            let attributes: [NSAttributedString.Key: Any] = [
                .font: Self.font,
                .foregroundColor: isSelected ? NSColor.white : NSColor.labelColor,
            ]
            let height = text.size(withAttributes: attributes).height
            text.draw(at: NSPoint(x: Self.textInset, y: top + (Self.rowHeight - height) / 2), withAttributes: attributes)
            // Hairline separators between rows, as in the system list.
            if row < rows - 1 && !isSelected && index + 1 != selected {
                NSColor.separatorColor.withAlphaComponent(0.35).setFill()
                NSRect(x: Self.textInset, y: top + Self.rowHeight - 0.5, width: rightEdge - Self.textInset * 2, height: 0.5).fill()
            }
        }
        if overflow {
            // A slim rounded scroll thumb on the right.
            let track = NSRect(x: bounds.width - 12, y: 8, width: 6, height: bounds.height - 16)
            let fraction = CGFloat(Self.visibleRows) / CGFloat(items.count)
            let offset = CGFloat(firstVisible) / CGFloat(items.count - Self.visibleRows)
            let thumbHeight = max(18, track.height * fraction)
            let thumb = NSRect(x: track.minX, y: track.minY + (track.height - thumbHeight) * offset, width: 6, height: thumbHeight)
            NSColor.labelColor.withAlphaComponent(0.45).setFill()
            NSBezierPath(roundedRect: thumb, xRadius: 3, yRadius: 3).fill()
        }
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if horizontal {
            let index = Int((point.x - 5) / cellWidth)
            if index >= 0 && index < items.count { onPick?(index) }
            return
        }
        let row = Int((point.y - 5) / Self.rowHeight)
        let index = firstVisible + row
        if row >= 0 && row < Self.visibleRows && index < items.count {
            onPick?(index)
        }
    }

    override func scrollWheel(with event: NSEvent) {
        if horizontal { return }
        let step = event.scrollingDeltaY > 0 ? -1 : event.scrollingDeltaY < 0 ? 1 : 0
        firstVisible = min(max(0, firstVisible + step), max(0, items.count - Self.visibleRows))
        needsDisplay = true
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
}
