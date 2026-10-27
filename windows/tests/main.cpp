// Shell tests: the scan code map (spec §2), and key sequences typed through it into the engine,
// as the text service does. Behavior itself is the engine's (engine/tests); this checks the wiring.
#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "jis_korean.h"
#include "json.h"
#include "key_map.h"

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

    std::printf(failures ? "%d failed\n" : "all passed\n", failures);
    return failures ? 1 : 0;
}
