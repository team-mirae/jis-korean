#include "key_map.h"

#include "jis_korean.h"

namespace jk {
namespace {

// Scan code set 1 → engine key, for keys that aren't extended (spec §2). 0xFF: not mapped.
constexpr std::uint8_t kNone = 0xFF;

struct ScanTable {
    std::uint8_t keys[0x80];
    constexpr ScanTable() : keys() {
        for (auto& k : keys) k = kNone;
        // Number row: 1 … 0 are 0x02 … 0x0B.
        for (int i = 0; i < 10; ++i) keys[0x02 + i] = static_cast<std::uint8_t>(JK_KEY_DIGIT1 + i);
        keys[0x0C] = JK_KEY_MINUS;
        keys[0x0D] = JK_KEY_EQUAL;
        keys[0x7D] = JK_KEY_INTLYEN;  // ¥, JIS only
        const std::uint8_t q_row[] = {JK_KEY_KEYQ, JK_KEY_KEYW, JK_KEY_KEYE, JK_KEY_KEYR,
                                      JK_KEY_KEYT, JK_KEY_KEYY, JK_KEY_KEYU, JK_KEY_KEYI,
                                      JK_KEY_KEYO, JK_KEY_KEYP, JK_KEY_BRACKETLEFT,
                                      JK_KEY_BRACKETRIGHT};
        for (int i = 0; i < 12; ++i) keys[0x10 + i] = q_row[i];
        const std::uint8_t a_row[] = {JK_KEY_KEYA, JK_KEY_KEYS, JK_KEY_KEYD, JK_KEY_KEYF,
                                      JK_KEY_KEYG, JK_KEY_KEYH, JK_KEY_KEYJ, JK_KEY_KEYK,
                                      JK_KEY_KEYL, JK_KEY_SEMICOLON, JK_KEY_QUOTE};
        for (int i = 0; i < 11; ++i) keys[0x1E + i] = a_row[i];
        keys[0x29] = JK_KEY_BACKQUOTE;  // US `; on JIS the 半角/全角 key, which Mode A passes on
        keys[0x2B] = JK_KEY_BACKSLASH;  // US \; JIS ]
        const std::uint8_t z_row[] = {JK_KEY_KEYZ, JK_KEY_KEYX, JK_KEY_KEYC,  JK_KEY_KEYV,
                                      JK_KEY_KEYB, JK_KEY_KEYN, JK_KEY_KEYM,  JK_KEY_COMMA,
                                      JK_KEY_PERIOD, JK_KEY_SLASH};
        for (int i = 0; i < 10; ++i) keys[0x2C + i] = z_row[i];
        keys[0x73] = JK_KEY_INTLRO;  // ろ, JIS only
        keys[0x56] = JK_KEY_INTLBACKSLASH;  // between left Shift and Z, ISO only (Mode C)
    }
};
constexpr ScanTable kScan;

}  // namespace

std::uint32_t EngineKey(unsigned key, unsigned scan, bool extended) {
    switch (key) {
        // A character injected as Unicode (SendInput KEYEVENTF_UNICODE): its "scan code" is the
        // character's low byte, not a position. It commits first and reaches the app as typed (#4).
        case vk::kPacket:
            return JK_KEY_OTHER;
        // A modifier on its own belongs to the next key: it must never commit (FKeyIME
        // docs/issues/shift-splits-the-composition.md). Caps Lock likewise.
        case vk::kShift: case vk::kLShift: case vk::kRShift:
        case vk::kControl: case vk::kLControl: case vk::kRControl:
        case vk::kMenu: case vk::kLMenu: case vk::kRMenu:
        case vk::kLWin: case vk::kRWin: case vk::kCapital:
            return JK_KEY_MODIFIER;
        case vk::kF6: return JK_KEY_F6;
        case vk::kF7: return JK_KEY_F7;
        case vk::kF8: return JK_KEY_F8;
        case vk::kF9: return JK_KEY_F9;
        case vk::kF10: return JK_KEY_F10;
        case vk::kSpace: return JK_KEY_SPACE;
        case vk::kReturn: return JK_KEY_ENTER;  // the keypad's Enter too
        case vk::kBack: return JK_KEY_BACKSPACE;
        case vk::kEscape: return JK_KEY_ESCAPE;
        case vk::kUp: return JK_KEY_ARROWUP;
        case vk::kDown: return JK_KEY_ARROWDOWN;
        case vk::kLeft: return JK_KEY_ARROWLEFT;
        case vk::kRight: return JK_KEY_ARROWRIGHT;
    }
    // Keypad keys, Home/End and the like are extended or outside the table: not ours.
    if (extended || scan >= 0x80) return JK_KEY_OTHER;
    const std::uint8_t k = kScan.keys[scan];
    return k == kNone ? JK_KEY_OTHER : k;
}

std::uint32_t KeyOf(unsigned vk, std::uintptr_t lparam, unsigned (*scan_of)(unsigned vk)) {
    unsigned scan = static_cast<unsigned>((lparam >> 16) & 0xFF);
    const bool extended = (lparam & (std::uintptr_t{1} << 24)) != 0;
    if (scan == 0 && vk != vk::kPacket && scan_of) scan = scan_of(vk) & 0xFF;
    return EngineKey(vk, scan, extended);
}

bool SameKeyDown(std::uintptr_t wparam, std::intptr_t lparam, std::int32_t time, std::uint64_t tick,
                 std::uintptr_t wparam2, std::intptr_t lparam2, std::int32_t time2, std::uint64_t tick2) {
    if (wparam != wparam2 || lparam != lparam2) return false;
    if (time == time2) return true;
    // A missed key-up (focus moved to another thread in between) must not swallow a later press.
    constexpr std::uint64_t kRetest = 500;
    const bool first_press = (lparam & (std::intptr_t{1} << 30)) == 0;
    return first_press && tick2 >= tick && tick2 - tick < kRetest;
}

}  // namespace jk
