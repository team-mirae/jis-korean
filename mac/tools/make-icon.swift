// Renders the menu bar icon: 한 cut out of a filled rounded square, as a black template image.
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

    // Apple's Korean input modes: a filled rounded square, the glyph cut out of it in a light weight.
    let box = NSBezierPath(
        roundedRect: NSRect(x: 0, y: 0, width: points, height: points),
        xRadius: points * 0.14, yRadius: points * 0.14)
    NSColor.black.setFill()
    box.fill()

    let font = NSFont.systemFont(ofSize: points * 0.72, weight: .regular)
    let text = NSAttributedString(string: "한", attributes: [.font: font])
    let line = CTLineCreateWithAttributedString(text)
    let ink = CTLineGetImageBounds(line, nil)
    let context = NSGraphicsContext.current!.cgContext
    context.setBlendMode(.destinationOut)
    context.textPosition = CGPoint(x: (points - ink.width) / 2 - ink.minX, y: (points - ink.height) / 2 - ink.minY)
    CTLineDraw(line, context)

    NSGraphicsContext.restoreGraphicsState()
    guard let png = rep.representation(using: .png, properties: [:]) else { fatalError("png") }
    try! png.write(to: URL(fileURLWithPath: path))
}

render(points: 16, scale: 1, to: "\(outDir)/icon.png")
render(points: 16, scale: 2, to: "\(outDir)/icon@2x.png")
