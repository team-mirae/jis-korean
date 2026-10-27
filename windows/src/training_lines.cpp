#include "training_lines.h"

#include <windows.h>

#include <algorithm>
#include <cmath>
#include <map>

namespace jk::training {

std::wstring Wide(const std::string& s) {
    if (s.empty()) return {};
    const int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    std::wstring w(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), w.data(), n);
    return w;
}

bool KoreanWord(const std::wstring& w) {
    return !w.empty() && std::all_of(w.begin(), w.end(), [](wchar_t c) { return c >= 0xAC00 && c <= 0xD7A3; });
}

bool EnglishWord(const std::wstring& w) {
    return !w.empty() && std::all_of(w.begin(), w.end(), [](wchar_t c) {
        return (c >= L'a' && c <= L'z') || (c >= L'A' && c <= L'Z');
    });
}

int ClampCount(double x, int lo, int hi, int fallback) {
    if (!std::isfinite(x)) return fallback;
    return static_cast<int>(std::clamp(x, static_cast<double>(lo), static_cast<double>(hi)));
}

std::wstring Lines(const json::Value* list, bool (*valid)(const std::wstring&)) {
    static const std::map<std::string, int> kConfidence = {{"tentative", 10}, {"likely", 20}, {"confirmed", 50}};
    constexpr int kVeryCommon = 1000;  // no count, no confidence (training-contract.md)
    std::wstring out;
    if (!list || list->type != json::Value::Type::kArray) return out;
    int n = 0;
    for (const json::Value& entry : list->array) {
        const json::Value* word = entry.Get("word");
        if (!word || !word->String()) continue;
        const std::wstring w = Wide(*word->String());
        if (!valid(w)) continue;
        int count = kVeryCommon;
        if (const json::Value* c = entry.Get("count"); c && c->Number()) {
            count = ClampCount(*c->Number(), 1, 1000000, kVeryCommon);
        } else if (const json::Value* conf = entry.Get("confidence"); conf && conf->String()) {
            const auto it = kConfidence.find(*conf->String());
            if (it != kConfidence.end()) count = it->second;
        }
        if (!out.empty()) out += L'\n';
        out += w + L'\t' + std::to_wstring(count);
        if (++n == 5000) break;
    }
    return out;
}

std::wstring AvoidLines(const json::Value* list, const char* language, bool (*valid)(const std::wstring&)) {
    std::wstring out;
    if (!list || list->type != json::Value::Type::kArray) return out;
    int n = 0;
    for (const json::Value& entry : list->array) {
        const json::Value* word = entry.Get("word");
        const json::Value* lang = entry.Get("language");
        if (!word || !word->String() || !lang || !lang->String() || *lang->String() != language) continue;
        const std::wstring w = Wide(*word->String());
        if (!valid(w)) continue;
        const json::Value* strength = entry.Get("strength");
        if (!out.empty()) out += L'\n';
        out += w;
        if (strength && strength->String() && *strength->String() == "strong") out += L"\tstrong";
        if (++n == 5000) break;
    }
    return out;
}

}  // namespace jk::training
