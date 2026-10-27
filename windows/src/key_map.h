// Windows key events → engine key ids (spec §2). Pure: no Windows headers, so it's unit-tested.
//
// Character keys go by scan code (physical position), never by the virtual key, which depends on
// the keyboard layout Windows has loaded for Korean. Keys whose position doesn't matter (Space,
// Enter, arrows, F-keys) go by virtual key, so injected input without scan codes still works.
#pragma once

#include <cstdint>

namespace jk {

// Virtual keys the map needs, as in winuser.h.
namespace vk {
inline constexpr unsigned kBack = 0x08, kTab = 0x09, kReturn = 0x0D, kShift = 0x10,
                          kControl = 0x11, kMenu = 0x12, kCapital = 0x14, kKana = 0x15,
                          kKanji = 0x19, kEscape = 0x1B, kSpace = 0x20, kLeft = 0x25,
                          kUp = 0x26, kRight = 0x27, kDown = 0x28, kLWin = 0x5B, kRWin = 0x5C,
                          kF6 = 0x75, kF7 = 0x76, kF8 = 0x77, kF9 = 0x78, kF10 = 0x79,
                          kLShift = 0xA0, kRShift = 0xA1, kLControl = 0xA2, kRControl = 0xA3,
                          kLMenu = 0xA4, kRMenu = 0xA5;
}  // namespace vk

// The engine key (JK_KEY_*) for a key-down with virtual key `vk`, scan code `scan` (lParam bits
// 16-23) and the extended-key flag (lParam bit 24).
std::uint32_t EngineKey(unsigned vk, unsigned scan, bool extended);

// Whether a key-down test (wparam, lParam, GetMessageTime, GetTickCount64) is the same key-down as
// an earlier one with no key-up of it in between, tested again (#5). A first press (lParam bit 30
// clear) can't come twice without a key-up, so the same lParam within a short while is the same
// one, even when the message time differs (a test before the message was taken reads the previous
// message's time). Auto-repeats share their lParam: those must have the same message time too.
bool SameKeyDown(std::uintptr_t wparam, std::intptr_t lparam, std::int32_t time, std::uint64_t tick,
                 std::uintptr_t wparam2, std::intptr_t lparam2, std::int32_t time2, std::uint64_t tick2);

}  // namespace jk
