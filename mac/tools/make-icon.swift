// Renders the menu bar icon: 한 in a rounded square, as a black template image.
// Usage: swift make-icon.swift <out-dir>   (writes icon.png and icon@2x.png)
import AppKit

let outDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."

func render(points: CGFloat, scale: CGFloat, to path: String) {
    let pixels = Int(points * scale)
    guard let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels, bitsPerSample: 8,
        samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
        bytesPerRow: 0, bitsPerPixel: 0)
    else { fatalError("bitmap") }
    rep.size = NSSize(width: points, height: points)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)

    let frame = NSRect(x: 0.5, y: 0.5, width: points - 1, height: points - 1)
    let box = NSBezierPath(roundedRect: frame, xRadius: points * 0.2, yRadius: points * 0.2)
    box.lineWidth = 1
    NSColor.black.setStroke()
    box.stroke()

    let font = NSFont.systemFont(ofSize: points * 0.62, weight: .semibold)
    let text = NSAttributedString(string: "한", attributes: [.font: font, .foregroundColor: NSColor.black])
    let size = text.size()
    text.draw(at: NSPoint(x: (points - size.width) / 2, y: (points - size.height) / 2))

    NSGraphicsContext.restoreGraphicsState()
    guard let png = rep.representation(using: .png, properties: [:]) else { fatalError("png") }
    try! png.write(to: URL(fileURLWithPath: path))
}

render(points: 16, scale: 1, to: "\(outDir)/icon.png")
render(points: 16, scale: 2, to: "\(outDir)/icon@2x.png")
