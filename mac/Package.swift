// swift-tools-version:5.9
// The macOS shell as a Swift package, for CodeQL only: code scanning traces Swift only through
// `xcodebuild` or `swift build`, and the Makefile calls swiftc. It compiles Sources/ with the
// Makefile's flags for the official build (`make ENGLISH=1`: predictions and the English modes),
// and links the engine `make ENGLISH=1 build/full-english/libjis_korean.a` builds first. It makes
// no app bundle: build and install with `make` (README.md).
import Foundation
import PackageDescription

let mac = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "JISKorean",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "JISKorean",
            path: "Sources",
            swiftSettings: [
                .define("PREDICT"),
                .define("ENGLISH"),
                .unsafeFlags(["-import-objc-header", "\(mac)/../engine/include/jis_korean.h"]),
            ],
            linkerSettings: [
                .linkedFramework("Cocoa"),
                .linkedFramework("InputMethodKit"),
                .linkedFramework("Carbon"),
                .linkedFramework("IOKit"),
                .unsafeFlags(["\(mac)/build/full-english/libjis_korean.a"]),
            ]
        )
    ]
)
