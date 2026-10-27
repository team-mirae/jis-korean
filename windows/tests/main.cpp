// Shell tests: the scan code map (spec §2), and key sequences typed through it into the engine,
// as the text service does. Behavior itself is the engine's (engine/tests); this checks the wiring.
#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "jis_korean.h"
#include "json.h"
#include "key_map.h"
#include "log.h"
#include "training_lines.h"

namespace {

int failures = 0;

void Check(bool ok, const char* what) {
    if (!ok) {
        std::printf("FAIL %s\n", what);
        ++failures;
    }
}

std::string Str(const std::uint8_t* p, size_t n) { return std::string(reinterpret_cast<const char*>(p), n); }

struct Press {
    unsigned vk;
    unsigned scan;
    bool shift = false;
};

// Types `presses` into a fresh engine in `mode`; returns everything committed + the preedit.
std::string Type(unsigned mode, const std::vector<Press>& presses, std::string* preedit = nullptr,
                 bool dedicated_symbols = true) {
    JkEngine* e = jk_new();
    jk_set_mode(e, mode);
    jk_set_dedicated_symbols(e, dedicated_symbols ? 1 : 0);
    std::string committed;
    for (const Press& p : presses) {
        jk_key(e, jk::EngineKey(p.vk, p.scan, false), p.shift ? JK_MOD_SHIFT : 0);
        committed += Str(jk_commit_ptr(e), jk_commit_len(e));
    }
    if (preedit) *preedit = Str(jk_preedit_ptr(e), jk_preedit_len(e));
    jk_free(e);
    return committed;
}

size_t Format(wchar_t* line, size_t size, size_t used, const wchar_t* format, ...) {
    va_list args;
    va_start(args, format);
    const size_t n = jk::log::Append(line, size, used, format, args);
    va_end(args);
    return n;
}

// The key code name of an engine key, as the engine spells it.
std::string Code(std::uint32_t key) { return Str(jk_key_code_ptr(key), jk_key_code_len(key)); }

}  // namespace

int main() {
    // Spec §2: scan codes → KeyboardEvent.code names, whatever the virtual key.
    struct Case {
        unsigned scan;
        const char* code;
    };
    const Case cases[] = {
        {0x02, "Digit1"}, {0x0B, "Digit0"}, {0x0C, "Minus"},       {0x0D, "Equal"},
        {0x7D, "IntlYen"}, {0x10, "KeyQ"},  {0x19, "KeyP"},        {0x1A, "BracketLeft"},
        {0x1B, "BracketRight"}, {0x1E, "KeyA"}, {0x26, "KeyL"},    {0x27, "Semicolon"},
        {0x28, "Quote"},  {0x2B, "Backslash"}, {0x2C, "KeyZ"},     {0x32, "KeyM"},
        {0x33, "Comma"},  {0x34, "Period"}, {0x35, "Slash"},       {0x73, "IntlRo"},
        {0x29, "Backquote"}, {0x56, "IntlBackslash"},
    };
    for (const Case& c : cases) {
        const std::uint32_t key = jk::EngineKey(0xE5 /* VK_PROCESSKEY: ignored */, c.scan, false);
        Check(key != JK_KEY_OTHER && Code(key) == c.code, c.code);
    }
    Check(jk::EngineKey(0x6F, 0x35, true) == JK_KEY_OTHER, "keypad / is not Slash");
    Check(jk::EngineKey(0x0D, 0x1C, true) == JK_KEY_ENTER, "keypad Enter is Enter");
    Check(jk::EngineKey(0x10, 0x2A, false) == JK_KEY_MODIFIER, "Shift alone");
    Check(jk::EngineKey(0x14, 0x3A, false) == JK_KEY_MODIFIER, "Caps Lock alone");
    Check(jk::EngineKey(0x75, 0x40, false) == JK_KEY_F6, "F6");
    Check(jk::EngineKey(0x79, 0x44, false) == JK_KEY_F10, "F10");
    Check(jk::EngineKey(0x24, 0x47, true) == JK_KEY_OTHER, "Home");
    Check(jk::EngineKey(0x15, 0x38, true) == JK_KEY_OTHER, "한/영 isn't an engine key");
    // #4: VK_PACKET carries a character's low byte where the scan code goes: never a position.
    for (unsigned scan : {0x20u, 0x2Cu, 0x30u, 0x10u, 0x56u, 0x73u, 0x7Du, 0x00u}) {
        Check(jk::EngineKey(0xE7, scan, false) == JK_KEY_OTHER, "VK_PACKET is never a letter key");
    }

    // #9: a key injected with a virtual key but scan code 0 gets its scan code from the layout.
    {
        auto scan_of = [](unsigned vk) -> unsigned { return vk == 'G' ? 0x22u : vk == 0xE7 ? 0x20u : 0u; };
        Check(jk::KeyOf('G', 0x00000001, scan_of) == JK_KEY_KEYG, "scan 0 + VK G → KeyG");
        Check(jk::KeyOf('G', 0x00220001, scan_of) == JK_KEY_KEYG, "a real scan code is kept");
        Check(jk::KeyOf('K', 0x00220001, scan_of) == JK_KEY_KEYG, "the scan code wins over the VK");
        Check(jk::KeyOf(0xE7, 0x00000001, scan_of) == JK_KEY_OTHER, "VK_PACKET doesn't take the scan-0 path");
        Check(jk::KeyOf(0x25, 0x014B0001, scan_of) == JK_KEY_ARROWLEFT, "extended arrows by VK");
        Check(jk::KeyOf('G', 0x00000001, nullptr) == JK_KEY_OTHER, "no lookup: unknown");
    }

    // #9: training file counts are clamped before the cast (1e300, -1e300, infinity).
    {
        Check(jk::training::ClampCount(1e300, 1, 1000000, 1000) == 1000000, "1e300 → the top");
        Check(jk::training::ClampCount(-1e300, 1, 1000000, 1000) == 1, "-1e300 → the bottom");
        Check(jk::training::ClampCount(std::nan(""), 1, 20, 1) == 1, "NaN → the default");
        Check(jk::training::ClampCount(HUGE_VAL, 1, 20, 1) == 1, "infinity → the default");
        Check(jk::training::ClampCount(7.9, 1, 20, 1) == 7, "in range");
        const auto list = jk::json::Parse(
            R"([{"word": "가나", "count": 1e300}, {"word": "다라", "count": -5e20}, {"word": "마바", "count": 1e999},)"
            R"( {"word": "abc", "count": 12}, {"word": "사아", "confidence": "likely"}, {"word": "자차"}])");
        Check(list.has_value(), "the vocabulary list parses");
        if (list) {
            Check(jk::training::Lines(&*list, jk::training::KoreanWord) ==
                      L"가나\t1000000\n다라\t1\n마바\t1000\n사아\t20\n자차\t1000",
                  "Lines clamps counts and skips words in the wrong script");
            Check(jk::training::Lines(&*list, jk::training::EnglishWord) == L"abc\t12", "English Lines");
        }
    }

    // #5: a key-down tested twice is recognised; separate presses and auto-repeats aren't.
    {
        const std::intptr_t press = 0x00220001, repeat = 0x40220001;  // KeyG: scan 0x22, bit 30 for repeats
        Check(jk::SameKeyDown(0x47, press, 100, 1000, 0x47, press, 100, 1000), "the same test twice");
        Check(jk::SameKeyDown(0x47, press, 90, 1000, 0x47, press, 100, 1003),
              "tested before the message was taken (previous message's time), then again");
        Check(!jk::SameKeyDown(0x47, press, 90, 1000, 0x47, press, 2000, 2000),
              "a later press after a missed key-up isn't swallowed");
        Check(!jk::SameKeyDown(0x47, repeat, 100, 1000, 0x47, repeat, 133, 1033), "auto-repeats each count");
        Check(jk::SameKeyDown(0x47, repeat, 133, 1033, 0x47, repeat, 133, 1035), "an auto-repeat tested twice");
        Check(!jk::SameKeyDown(0x47, press, 100, 1000, 0x4B, 0x00250001, 100, 1000), "another key");
    }

    // Typed through the map: G K S → 한 (unconfirmed), Space commits it with a space.
    const Press g = {0x47, 0x22}, k = {0x4B, 0x25}, s = {0x53, 0x1F}, space = {0x20, 0x39};
    std::string preedit;
    Check(Type(0, {g, k, s}, &preedit).empty() && preedit == "한", "G K S → 한");
    Check(Type(0, {g, k, s, space}) == "한 ", "한 Space");
    // Spec §6.2: F10 → gks.
    Check((Type(0, {g, k, s, {0x79, 0x44}}, &preedit), preedit == "gks"), "한 F10 → gks");
    // Shift+Z is F10 too (spec §3.4).
    Check((Type(0, {g, k, s, {0x5A, 0x2C, true}}, &preedit), preedit == "gks"), "한 Shift+Z → gks");
    // Number row: 1 → ㅃ; Mode B Shift+[ → 「 (spec §3.6).
    Check((Type(0, {{0x31, 0x02}}, &preedit), preedit == "ㅃ"), "1 → ㅃ");
    Check((Type(1, {{0xDB, 0x1A, true}}, &preedit), preedit == "「"), "Mode B Shift+[ → 「");
    // Mode A: the @ key (US [) is ' (spec §3.2).
    Check((Type(0, {{0xDB, 0x1A}}, &preedit), preedit == "'"), "Mode A @ → '");
    // Mode C (spec §3.6): the ISO extra key types · and —, [ types 「 with Shift; with Dedicated
    // symbol keys off they're UK ISO's \ | and { again.
    Check(jk_mode_count() >= 3, "three modes");
    // Mode ids have gaps (spec §3.6.1): the menu lists them by index, and refuses unknown ids.
    bool listed_c = false;
    for (std::uint32_t i = 0; i < jk_mode_count(); ++i) listed_c |= jk_mode_id_at(i) == 2;
    Check(listed_c && jk_mode_id_at(jk_mode_count()) == UINT32_MAX, "jk_mode_id_at lists Mode C");
    Check(Str(jk_mode_code_ptr(2), jk_mode_code_len(2)) == "C", "Mode C's code is C");
    {
        JkEngine* e = jk_new();
        Check(jk_set_mode(e, 2) == 1 && jk_set_mode(e, 4242) == 0, "unknown mode ids are refused");
        jk_free(e);
    }
    const Press iso = {0xE2, 0x56}, iso_shift = {0xE2, 0x56, true}, bracket_shift = {0xDB, 0x1A, true};
    Check((Type(2, {iso, iso_shift, bracket_shift}, &preedit), preedit == "\xC2\xB7\xE2\x80\x94\xE3\x80\x8C"),
          "Mode C: extra key · —, Shift+[ 「");
    Check((Type(2, {iso, iso_shift, bracket_shift}, &preedit, false), preedit == "\\|{"),
          "Mode C, dedicated symbol keys off: \\ | {");
    Check((Type(1, {iso}, &preedit), preedit.empty()), "the ISO extra key isn't in Mode B");

    // The training-file reader (training.cpp): the repo's example, escapes, and bad input.
    {
        std::ifstream file(JK_DOCS_DIR "/training-example.json", std::ios::binary);
        std::stringstream text;
        text << file.rdbuf();
        const auto example = jk::json::Parse(text.str());
        Check(example && example->Get("format") && *example->Get("format")->String() == "jis-korean-training",
              "docs/training-example.json parses");
        Check(example && example->Get("version") && example->Get("version")->Number() == 1.0, "version 1");
        const auto escaped = jk::json::Parse(R"({"a": "\uD55C\u0041\n", "b": [1, -2.5e1, true, null]})");
        Check(escaped && *escaped->Get("a")->String() == "\xED\x95\x9C" "A\n", "\\u escapes");
        Check(escaped && escaped->Get("b")->array.size() == 4 && escaped->Get("b")->array[1].number == -25,
              "arrays and numbers");
        Check(!jk::json::Parse("{\"a\": }") && !jk::json::Parse("[1,") && !jk::json::Parse("\"\\uD800\""),
              "bad input is rejected");
        std::string deep(1000, '[');
        Check(!jk::json::Parse(deep), "deep nesting is rejected, not a stack overflow");
    }

    // ABC's English layouts (spec §3.6.2, §3.6.4, §8.4): the shell types jk_legend_in.
    {
        Check(!jk_mode_is_english(0) && !jk_mode_is_english(1) && !jk_mode_is_english(2) &&
                  !jk_mode_is_english(4242),
              "the Korean modes aren't English");
        const std::uint32_t digit2 = jk::EngineKey(0x32, 0x03, false), key_s = jk::EngineKey(0x53, 0x1F, false);
        const std::uint32_t ro = jk::EngineKey(0xE2, 0x73, false);
        Check(jk_mode_is_english(3) && jk_legend_in(digit2, 1, 0, 3) == U'"', "JIS English: Shift+2 is \"");
        Check(jk_mode_is_english(9) && jk_legend_in(digit2, 1, 0, 9) == U'@', "US English: Shift+2 is @");
        Check(jk_mode_is_english(7) && jk_legend_in(key_s, 0, 0, 7) == U'o', "JIS Dvorak: the S key is o");
        Check(jk_mode_is_english(8) && jk_legend_in(key_s, 0, 0, 8) == U'r', "JIS Colemak: the S key is r");
        Check(jk_mode_is_english(10) && jk_legend_in(ro, 0, 0, 10) == U'\\', "UK English: ろ is \\");
    }

    // The log truncates an overlong line instead of ending the process (log.cpp).
    {
        wchar_t line[16];
        const std::wstring big(100, L'x');
        size_t n = Format(line, 16, 3, L"%s", big.c_str());
        Check(n == 15 && line[14] == L'\n', "an overlong log line is truncated");
        n = Format(line, 16, 0, L"ab%d", 5);
        Check(n == 4 && std::wstring(line, n) == L"ab5\n", "a log line that fits");
    }

    std::printf(failures ? "%d failed\n" : "all passed\n", failures);
    return failures ? 1 : 0;
}
