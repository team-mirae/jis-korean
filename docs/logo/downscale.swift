// SPDX-License-Identifier: MIT
// Scales a large rendered PNG down to an icon size with high-quality filtering, so curves keep
// smooth (anti-aliased) edges. sips's default resampling left them stair-stepped at 16 and 32 px.
// Usage: swift downscale.swift <in.png> <out.png> <pixels>
import AppKit

let arguments = CommandLine.arguments
guard arguments.count == 4, let pixels = Int(arguments[3]),
      let source = NSImage(contentsOfFile: arguments[1])?.cgImage(forProposedRect: nil, context: nil, hints: nil)
else {
    FileHandle.standardError.write("usage: swift downscale.swift <in.png> <out.png> <pixels>\n".data(using: .utf8)!)
    exit(2)
}

// Halve repeatedly first, then the last step: each step averages neighbouring pixels, which keeps
// thin transparent gaps and curves smooth instead of sampling across them.
func scaled(_ image: CGImage, to size: Int) -> CGImage {
    let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!,
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    context.interpolationQuality = .high
    context.draw(image, in: CGRect(x: 0, y: 0, width: size, height: size))
    return context.makeImage()!
}

var image = source
while image.width / 2 >= pixels * 2 {
    image = scaled(image, to: image.width / 2)
}
image = scaled(image, to: pixels)

let rep = NSBitmapImageRep(cgImage: image)
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: arguments[2]))
