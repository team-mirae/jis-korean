import Carbon.HIToolbox

/// Physical macOS key codes → engine key ids (spec §2), as ANSI and JIS keyboards send them.
/// Positions are the same on every board except for ¥ and ろ, which only JIS has, ` (left of 1),
/// which ANSI and ISO have, and the extra key between left Shift and Z, which only ISO has.
/// ISO keyboards send two of these swapped: see [isoKeyMap].
let keyMap: [Int: UInt32] = [
    kVK_ANSI_Grave: UInt32(JK_KEY_BACKQUOTE),
    kVK_ISO_Section: UInt32(JK_KEY_INTLBACKSLASH),
    kVK_ANSI_1: UInt32(JK_KEY_DIGIT1), kVK_ANSI_2: UInt32(JK_KEY_DIGIT2), kVK_ANSI_3: UInt32(JK_KEY_DIGIT3),
    kVK_ANSI_4: UInt32(JK_KEY_DIGIT4), kVK_ANSI_5: UInt32(JK_KEY_DIGIT5), kVK_ANSI_6: UInt32(JK_KEY_DIGIT6),
    kVK_ANSI_7: UInt32(JK_KEY_DIGIT7), kVK_ANSI_8: UInt32(JK_KEY_DIGIT8), kVK_ANSI_9: UInt32(JK_KEY_DIGIT9),
    kVK_ANSI_0: UInt32(JK_KEY_DIGIT0), kVK_ANSI_Minus: UInt32(JK_KEY_MINUS), kVK_ANSI_Equal: UInt32(JK_KEY_EQUAL),
    kVK_JIS_Yen: UInt32(JK_KEY_INTLYEN),

    kVK_ANSI_Q: UInt32(JK_KEY_KEYQ), kVK_ANSI_W: UInt32(JK_KEY_KEYW), kVK_ANSI_E: UInt32(JK_KEY_KEYE),
    kVK_ANSI_R: UInt32(JK_KEY_KEYR), kVK_ANSI_T: UInt32(JK_KEY_KEYT), kVK_ANSI_Y: UInt32(JK_KEY_KEYY),
    kVK_ANSI_U: UInt32(JK_KEY_KEYU), kVK_ANSI_I: UInt32(JK_KEY_KEYI), kVK_ANSI_O: UInt32(JK_KEY_KEYO),
    kVK_ANSI_P: UInt32(JK_KEY_KEYP),
    kVK_ANSI_LeftBracket: UInt32(JK_KEY_BRACKETLEFT), kVK_ANSI_RightBracket: UInt32(JK_KEY_BRACKETRIGHT),

    kVK_ANSI_A: UInt32(JK_KEY_KEYA), kVK_ANSI_S: UInt32(JK_KEY_KEYS), kVK_ANSI_D: UInt32(JK_KEY_KEYD),
    kVK_ANSI_F: UInt32(JK_KEY_KEYF), kVK_ANSI_G: UInt32(JK_KEY_KEYG), kVK_ANSI_H: UInt32(JK_KEY_KEYH),
    kVK_ANSI_J: UInt32(JK_KEY_KEYJ), kVK_ANSI_K: UInt32(JK_KEY_KEYK), kVK_ANSI_L: UInt32(JK_KEY_KEYL),
    kVK_ANSI_Semicolon: UInt32(JK_KEY_SEMICOLON), kVK_ANSI_Quote: UInt32(JK_KEY_QUOTE),
    kVK_ANSI_Backslash: UInt32(JK_KEY_BACKSLASH),

    kVK_ANSI_Z: UInt32(JK_KEY_KEYZ), kVK_ANSI_X: UInt32(JK_KEY_KEYX), kVK_ANSI_C: UInt32(JK_KEY_KEYC),
    kVK_ANSI_V: UInt32(JK_KEY_KEYV), kVK_ANSI_B: UInt32(JK_KEY_KEYB), kVK_ANSI_N: UInt32(JK_KEY_KEYN),
    kVK_ANSI_M: UInt32(JK_KEY_KEYM),
    kVK_ANSI_Comma: UInt32(JK_KEY_COMMA), kVK_ANSI_Period: UInt32(JK_KEY_PERIOD), kVK_ANSI_Slash: UInt32(JK_KEY_SLASH),
    kVK_JIS_Underscore: UInt32(JK_KEY_INTLRO),

    kVK_F6: UInt32(JK_KEY_F6), kVK_F7: UInt32(JK_KEY_F7), kVK_F8: UInt32(JK_KEY_F8),
    kVK_F9: UInt32(JK_KEY_F9), kVK_F10: UInt32(JK_KEY_F10),

    kVK_Space: UInt32(JK_KEY_SPACE), kVK_Return: UInt32(JK_KEY_ENTER), kVK_ANSI_KeypadEnter: UInt32(JK_KEY_ENTER),
    kVK_Delete: UInt32(JK_KEY_BACKSPACE),
    kVK_Escape: UInt32(JK_KEY_ESCAPE),
    kVK_UpArrow: UInt32(JK_KEY_ARROWUP), kVK_DownArrow: UInt32(JK_KEY_ARROWDOWN),
    kVK_LeftArrow: UInt32(JK_KEY_ARROWLEFT), kVK_RightArrow: UInt32(JK_KEY_ARROWRIGHT),
]

/// [keyMap] as ISO keyboards send it: macOS swaps the codes of the key left of 1 (kVK_ISO_Section,
/// § on a Mac UK board) and the extra key left of Z (kVK_ANSI_Grave) when the keyboard is ISO.
let isoKeyMap: [Int: UInt32] = keyMap.merging([
    kVK_ISO_Section: UInt32(JK_KEY_BACKQUOTE),
    kVK_ANSI_Grave: UInt32(JK_KEY_INTLBACKSLASH),
]) { $1 }

/// Whether the keyboard typed on last is ISO, as macOS reports it.
func typingOnISO() -> Bool {
    KBGetLayoutType(Int16(LMGetKbdType())) == OSType(kKeyboardISO)
}

func engineKey(_ keyCode: UInt16) -> UInt32 {
    (typingOnISO() ? isoKeyMap : keyMap)[Int(keyCode)] ?? UInt32(JK_KEY_OTHER)
}
