// The training file's word lists as the settings keep them (docs/training-contract.md): pure
// functions of the parsed JSON, so they're unit-tested (tests/main.cpp).
#pragma once

#include <string>

#include "json.h"

namespace jk::training {

// The word lists accept whole Hangul syllables (가-힣) or ASCII letters only, as on macOS.
bool KoreanWord(const std::wstring& w);
bool EnglishWord(const std::wstring& w);

// A JSON number as an int in [lo, hi]; `fallback` for NaN or infinity. Clamped before the cast:
// casting an out-of-range double (1e300) to int is undefined.
int ClampCount(double x, int lo, int hi, int fallback);

// `word<TAB>count` lines from a vocabulary list, at most 5000. Counts are clamped to 1-1000000.
std::wstring Lines(const json::Value* list, bool (*valid)(const std::wstring&));
// `word` or `word<TAB>strong` lines from the avoid list for one language, at most 5000.
std::wstring AvoidLines(const json::Value* list, const char* language, bool (*valid)(const std::wstring&));

std::wstring Wide(const std::string& s);  // UTF-8 → UTF-16

}  // namespace jk::training
