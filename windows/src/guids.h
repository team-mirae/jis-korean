// JIS Korean identifiers. These GUIDs are permanent: they end up in users' registries and
// language lists (InputMethodTips "0412:{CLSID}{PROFILE}"). Never change them.
#pragma once

#include <guiddef.h>

namespace jk {

// The TIP's COM class. {D10651AD-51EA-4DBA-8BFF-1905261B26A1}
inline constexpr GUID kClsidTextService = {
    0xd10651ad, 0x51ea, 0x4dba, {0x8b, 0xff, 0x19, 0x05, 0x26, 0x1b, 0x26, 0xa1}};

// The one language profile, ko-KR. The mode (spec §3.6) is a setting in its menu, not a profile
// per mode as on macOS (spec §8.4). {669E0E87-FA58-46F6-96E1-0890A0224404}
inline constexpr GUID kProfile = {
    0x669e0e87, 0xfa58, 0x46f6, {0x96, 0xe1, 0x08, 0x90, 0xa0, 0x22, 0x44, 0x04}};

// The input indicator's mode button (Windows 8+). The SDK headers don't declare it; the value is
// Windows' own (as in Microsoft's SampleIME). {2C77A81E-41CC-4178-A3A7-5F8A987568E6}
inline constexpr GUID kLangBarItemInputMode = {
    0x2c77a81e, 0x41cc, 0x4178, {0xa3, 0xa7, 0x5f, 0x8a, 0x98, 0x75, 0x68, 0xe6}};

// Display attribute of unconfirmed text (underline). {9C43BCAF-FECF-4CFA-8E70-F10D48C7F946}
inline constexpr GUID kDisplayAttributeInput = {
    0x9c43bcaf, 0xfecf, 0x4cfa, {0x8e, 0x70, 0xf1, 0x0d, 0x48, 0xc7, 0xf9, 0x46}};

// Display attribute of a reading shown in place of the typed one: a hanja candidate or a
// prediction (thick underline). {49F0833E-36C7-4BEA-9D67-78A9382AED58}
inline constexpr GUID kDisplayAttributeConverted = {
    0x49f0833e, 0x36c7, 0x4bea, {0x9d, 0x67, 0x78, 0xa9, 0x38, 0x2a, 0xed, 0x58}};

// Global compartment (shared by every app in the session, never stored): nonzero while 한/영 has
// switched JIS Korean to English. {51678EF8-4A49-4832-A994-AC88AB0F76A6}
inline constexpr GUID kCompartmentEnglish = {
    0x51678ef8, 0x4a49, 0x4832, {0xa9, 0x94, 0xac, 0x88, 0xab, 0x0f, 0x76, 0xa6}};

// The text property holding a field's input scopes (InputScope.h's GUID_PROP_INPUTSCOPE, which
// no import library defines). {1713DD5A-68E7-4A5B-9AF6-592A595C778D}
inline constexpr GUID kPropInputScope = {
    0x1713dd5a, 0x68e7, 0x4a5b, {0x9a, 0xf6, 0x59, 0x2a, 0x59, 0x5c, 0x77, 0x8d}};

inline constexpr unsigned short kLangIdKorean = 0x0412;

}  // namespace jk
