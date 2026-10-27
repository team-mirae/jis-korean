// Builds one mode's .keylayout: the layout Keyboard Viewer shows while that mode is active (spec
// §8.1). The unshifted and Shift layers carry the mode's legends, from the engine
// (`cargo run --example legends <mode id>`, by key id) through Sources/KeyMap.swift (key code → key
// id). Every other layer, and every key the engine doesn't label, is ABC's on the keyboard being
// typed on (one key map set each for JIS, ANSI and ISO), read from this Mac: Cmd and Ctrl shortcuts
// and Option characters stay exactly as they are, whichever keyboard the mode is used on. One
// exception: on JIS, the key left of 1 (半角/全角) types nothing in a mode that doesn't label it.
// Usage: make-keylayout <legends.tsv> <out.keylayout> <name> <JIS|ANSI|ISO> <id> [caps]   (built by the
// Makefile with KeyMap.swift and the engine header; name and id must be unique among the bundle's
// layouts). `caps`: Caps Lock types the letters upper-case, for an English mode (JIS English); the
// Korean modes leave it out, so Caps Lock changes nothing there.

import Carbon
import Foundation

let arguments = CommandLine.arguments
guard [6, 7].contains(arguments.count), arguments.count == 6 || arguments[6] == "caps", let tsv = try? String(contentsOfFile: arguments[1], encoding: .utf8),
      ["JIS", "ANSI", "ISO"].contains(arguments[4]), let layoutID = Int(arguments[5])
else {
    FileHandle.standardError.write("usage: make-keylayout <legends.tsv> <out.keylayout> <name> <JIS|ANSI|ISO> <id> [caps]\n".data(using: .utf8)!)
    exit(2)
}
// The keyboard argument names the mode's own keyboard; the layout covers all three.
let name = arguments[3]
let caps = arguments.count == 7

// key id → (unshifted, shifted)
var legends: [UInt32: (String, String)] = [:]
for line in tsv.split(separator: "\n") {
    let fields = line.split(separator: "\t", omittingEmptySubsequences: false).map(String.init)
    if fields.count == 3, let id = UInt32(fields[0]) {
        legends[id] = (fields[1], fields[2])
    }
}

// One key map set per keyboard family, each with ABC as macOS translates it on that family: a
// mode is used on any keyboard (Mode A on a MacBook's ANSI board, say), and the Cmd, Ctrl and
// Option layers must stay that keyboard's ABC there, or shortcuts move (Cmd+] would go back).
let families: [(name: String, type: OSType)] = [("JIS", OSType(kKeyboardJIS)), ("ANSI", OSType(kKeyboardANSI)), ("ISO", OSType(kKeyboardISO))]
/// A keyboard type of each family, to translate ABC with.
var familyType: [String: UInt32] = [:]
/// Keyboard types 0–255 in runs of one family: the layout's <layout> entries.
var runs: [(first: Int, last: Int, family: String)] = []
for t in 0..<256 {
    let match = families.first { $0.type == KBGetLayoutType(Int16(t)) }
    if let match, familyType[match.name] == nil {
        familyType[match.name] = UInt32(t)
    }
    // Types macOS doesn't place in a family are ANSI, as ABC treats them.
    let family = match?.name ?? "ANSI"
    if let last = runs.last, last.family == family, last.last == t - 1 {
        runs[runs.count - 1].last = t
    } else {
        runs.append((t, t, family))
    }
}
let abc = (TISCreateInputSourceList([kTISPropertyInputSourceID as String: "com.apple.keylayout.ABC"] as CFDictionary, true)
    .takeRetainedValue() as! [TISInputSource])[0]
let data = Unmanaged<CFData>.fromOpaque(TISGetInputSourceProperty(abc, kTISPropertyUnicodeKeyLayoutData)).takeUnretainedValue() as Data
func abcOutput(_ code: Int, _ modifiers: Int, _ keyboardType: UInt32) -> String {
    data.withUnsafeBytes { raw -> String in
        let layout = raw.baseAddress!.assumingMemoryBound(to: UCKeyboardLayout.self)
        var dead: UInt32 = 0
        var length = 0
        var chars = [UniChar](repeating: 0, count: 8)
        UCKeyTranslate(layout, UInt16(code), UInt16(kUCKeyActionDown), UInt32((modifiers >> 8) & 0xFF), keyboardType,
                       OptionBits(kUCKeyTranslateNoDeadKeysBit), &dead, 8, &length, &chars)
        return String(utf16CodeUnits: chars, count: length)
    }
}

/// XML attribute text: everything outside printable ASCII letters and digits as a character reference.
func escaped(_ s: String) -> String {
    s.unicodeScalars.map { u in
        (u.properties.isAlphabetic && u.isASCII) || ("0"..."9").contains(Character(u)) ? String(u) : String(format: "&#x%04X;", u.value)
    }.joined()
}

// Layers in modifierMap order: plain, Shift, Option, Shift+Option, Cmd, Ctrl, and with `caps`, Caps Lock
// (the plain layer with its letters upper-case).
var layers = [0, shiftKey, optionKey, shiftKey | optionKey, cmdKey, controlKey]
if caps {
    layers.append(alphaLock)
}
// The plain layer's select: with `caps` it no longer takes Caps Lock, which has its own.
let plain = caps ? "" : "caps?"
let capsSelect = caps ? "\n    <keyMapSelect mapIndex=\"6\"><modifier keys=\"caps\"/></keyMapSelect>" : ""
var xml = """
<?xml version="1.1" encoding="UTF-8"?>
<!DOCTYPE keyboard SYSTEM "file://localhost/System/Library/DTDs/KeyboardLayout.dtd">
<!-- Generated by mac/tools/make-keylayout.swift from the engine's layout; don't edit. -->
<keyboard group="126" id="\(layoutID)" name="\(name)" maxout="3">
  <layouts>
\(runs.map { "    <layout first=\"\($0.first)\" last=\"\($0.last)\" mapSet=\"\($0.family)\" modifiers=\"Modifiers\"/>" }.joined(separator: "\n"))
  </layouts>
  <modifierMap id="Modifiers" defaultIndex="0">
    <keyMapSelect mapIndex="0"><modifier keys="\(plain)"/></keyMapSelect>
    <keyMapSelect mapIndex="1"><modifier keys="anyShift caps?"/></keyMapSelect>
    <keyMapSelect mapIndex="2"><modifier keys="anyOption caps?"/></keyMapSelect>
    <keyMapSelect mapIndex="3"><modifier keys="anyShift anyOption caps?"/></keyMapSelect>
    <keyMapSelect mapIndex="4"><modifier keys="command anyShift? anyOption? caps? anyControl?"/></keyMapSelect>
    <keyMapSelect mapIndex="5"><modifier keys="anyControl anyShift? anyOption? caps?"/></keyMapSelect>\(capsSelect)
  </modifierMap>

"""
for family in families.map(\.name) {
    let keyboardType = familyType[family] ?? 0
    // The engine's key ids for this family's key codes: ISO boards send two keys swapped.
    let ids = family == "ISO" ? isoKeyMap : keyMap
    xml += "  <keyMapSet id=\"\(family)\">\n"
    for (index, modifiers) in layers.enumerated() {
        xml += "    <keyMap index=\"\(index)\">\n"
        for code in 0..<128 {
            var out = abcOutput(code, modifiers, keyboardType)
            if index < 2 || index == 6, let id = ids[code], let legend = legends[id] {
                out = index == 1 ? legend.1 : legend.0
                if index == 6, out.count == 1, out.first!.isASCII, out.first!.isLetter {
                    out = out.uppercased()
                }
            } else if index < 2 || index == 6, family == "JIS", code == kVK_ANSI_Grave {
                // On a JIS board this key is 半角/全角, a language key: a mode that doesn't map it
                // (Mode A, spec §3.6) commits and types nothing, not ABC's ` (#12).
                out = ""
            }
            if !out.isEmpty {
                xml += "      <key code=\"\(code)\" output=\"\(escaped(out))\"/>\n"
            }
        }
        xml += "    </keyMap>\n"
    }
    xml += "  </keyMapSet>\n"
}
xml += "</keyboard>\n"
try! xml.write(toFile: arguments[2], atomically: true, encoding: .utf8)
