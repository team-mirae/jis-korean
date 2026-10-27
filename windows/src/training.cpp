#include "training.h"

#ifdef JK_PREDICT

#include <windows.h>
#include <shellapi.h>
#include <shlobj.h>

#include <algorithm>
#include <map>
#include <mutex>
#include <string>

#include "json.h"
#include "log.h"
#include "settings.h"
#include "shared_data.h"

namespace jk::training {
namespace {

constexpr wchar_t kAppliedDate[] = L"TrainingFileDate";
constexpr wchar_t kKoreanWords[] = L"TrainingWords.Korean";
constexpr wchar_t kEnglishWords[] = L"TrainingWords.English";
constexpr wchar_t kKoreanAvoid[] = L"TrainingAvoid.Korean";
constexpr wchar_t kEnglishAvoid[] = L"TrainingAvoid.English";

std::wstring Folder() {
    PWSTR path = nullptr;
    std::wstring folder;
    if (SUCCEEDED(SHGetKnownFolderPath(FOLDERID_RoamingAppData, 0, nullptr, &path))) {
        folder = std::wstring(path) + L"\\JIS Korean";
    }
    CoTaskMemFree(path);
    return folder;
}

std::wstring Wide(const std::string& s) {
    if (s.empty()) return {};
    const int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    std::wstring w(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), w.data(), n);
    return w;
}

std::string Narrow(const std::wstring& w) {
    if (w.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, w.data(), static_cast<int>(w.size()), nullptr, 0,
                                      nullptr, nullptr);
    std::string s(static_cast<size_t>(n), '\0');
    WideCharToMultiByte(CP_UTF8, 0, w.data(), static_cast<int>(w.size()), s.data(), n, nullptr, nullptr);
    return s;
}

// The word lists accept whole Hangul syllables (가-힣) or ASCII letters only, as on macOS.
bool KoreanWord(const std::wstring& w) {
    return !w.empty() && std::all_of(w.begin(), w.end(), [](wchar_t c) { return c >= 0xAC00 && c <= 0xD7A3; });
}
bool EnglishWord(const std::wstring& w) {
    return !w.empty() && std::all_of(w.begin(), w.end(), [](wchar_t c) {
        return (c >= L'a' && c <= L'z') || (c >= L'A' && c <= L'Z');
    });
}

// `word<TAB>count` lines from a vocabulary list (training-contract.md), at most 5000.
std::wstring Lines(const json::Value* list, bool (*valid)(const std::wstring&)) {
    static const std::map<std::string, int> kConfidence = {{"tentative", 10}, {"likely", 20}, {"confirmed", 50}};
    std::wstring out;
    if (!list || list->type != json::Value::Type::kArray) return out;
    int n = 0;
    for (const json::Value& entry : list->array) {
        const json::Value* word = entry.Get("word");
        if (!word || !word->String()) continue;
        const std::wstring w = Wide(*word->String());
        if (!valid(w)) continue;
        int count = 1000;
        if (const json::Value* c = entry.Get("count"); c && c->Number()) {
            count = static_cast<int>(*c->Number());
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

// `word` or `word<TAB>strong` lines from the avoid list (training-contract.md) for one language, at
// most 5000.
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

ULONGLONG FileTime(const std::wstring& file, bool* exists) {
    WIN32_FILE_ATTRIBUTE_DATA attributes = {};
    *exists = GetFileAttributesExW(file.c_str(), GetFileExInfoStandard, &attributes) != FALSE;
    return (static_cast<ULONGLONG>(attributes.ftLastWriteTime.dwHighDateTime) << 32) |
           attributes.ftLastWriteTime.dwLowDateTime;
}

}  // namespace

bool Changed() {
    bool exists = false;
    const ULONGLONG time = FileTime(Folder() + L"\\training.json", &exists);
    const auto applied = settings::ReadString(kAppliedDate);
    return exists ? applied != std::to_wstring(time) : applied.has_value();
}

const JkPredictor* Predictor() {
    static std::mutex mutex;
    static std::wstring words;  // the words `personal` was built with
    static JkPredictor* personal = nullptr;
    const std::wstring korean = settings::ReadString(kKoreanWords).value_or(L"");
    const std::wstring english = settings::ReadString(kEnglishWords).value_or(L"");
    const std::wstring avoidKorean = settings::ReadString(kKoreanAvoid).value_or(L"");
    const std::wstring avoidEnglish = settings::ReadString(kEnglishAvoid).value_or(L"");
    if (korean.empty() && english.empty() && avoidKorean.empty() && avoidEnglish.empty()) return SharedPredictor();
    const std::wstring key = korean + L'\0' + english + L'\0' + avoidKorean + L'\0' + avoidEnglish;
    std::lock_guard<std::mutex> lock(mutex);
    if (!personal || key != words) {
        const std::string k = Narrow(korean), e = Narrow(english);
        JkPredictor* built = jk_predictor_with_words(
            SharedPredictor(), reinterpret_cast<const std::uint8_t*>(k.data()), k.size(),
            reinterpret_cast<const std::uint8_t*>(e.data()), e.size());
        if (built && !(avoidKorean.empty() && avoidEnglish.empty())) {
            const std::string ak = Narrow(avoidKorean), ae = Narrow(avoidEnglish);
            JkPredictor* avoided = jk_predictor_with_avoided(
                built, reinterpret_cast<const std::uint8_t*>(ak.data()), ak.size(),
                reinterpret_cast<const std::uint8_t*>(ae.data()), ae.size());
            if (avoided) {
                jk_predictor_free(built);
                built = avoided;
            }
        }
        if (personal) jk_predictor_free(personal);  // engines keep their own reference
        personal = built;
        words = key;
    }
    return personal ? personal : SharedPredictor();
}

void ApplyIfChanged(JkEngine* engine) {
    const std::wstring file = Folder() + L"\\training.json";
    WIN32_FILE_ATTRIBUTE_DATA attributes = {};
    if (Folder().empty() || !GetFileAttributesExW(file.c_str(), GetFileExInfoStandard, &attributes)) {
        if (settings::ReadString(kAppliedDate)) {  // the file is gone: forget its words
            settings::Delete(kAppliedDate);
            settings::Delete(kKoreanWords);
            settings::Delete(kEnglishWords);
            settings::Delete(kKoreanAvoid);
            settings::Delete(kEnglishAvoid);
        }
        return;
    }
    const ULONGLONG time = (static_cast<ULONGLONG>(attributes.ftLastWriteTime.dwHighDateTime) << 32) |
                           attributes.ftLastWriteTime.dwLowDateTime;
    const std::wstring date = std::to_wstring(time);
    if (settings::ReadString(kAppliedDate) == date) return;
    settings::WriteString(kAppliedDate, date);

    std::string text;
    HANDLE handle = CreateFileW(file.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr,
                                OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (handle != INVALID_HANDLE_VALUE) {
        LARGE_INTEGER size;
        if (GetFileSizeEx(handle, &size) && size.QuadPart < (64 << 20)) {
            text.resize(static_cast<size_t>(size.QuadPart));
            DWORD read = 0;
            if (!ReadFile(handle, text.data(), static_cast<DWORD>(text.size()), &read, nullptr)) read = 0;
            text.resize(read);
        }
        CloseHandle(handle);
    }
    const std::optional<json::Value> json = json::Parse(text);
    const json::Value* format = json ? json->Get("format") : nullptr;
    const json::Value* version = json ? json->Get("version") : nullptr;
    if (!format || !format->String() || *format->String() != "jis-korean-training" || !version ||
        version->Number() != 1.0) {
        log::Write(L"training.json isn't a version 1 training file; ignored");
        return;
    }
    static const std::map<std::string, const wchar_t*> kWeights = {
        {"korean", L"Korean"}, {"english", L"English"},   {"implied", L"Implied"},
        {"mixed", L"Mixed"},   {"two-set", L"TwoSet"},    {"keep-finals", L"KeepFinals"},
        {"hanja", L"Hanja"},   {"implied-l", L"ImpliedL"},
    };
    if (const json::Value* weights = json->Get("weights"); weights && weights->type == json::Value::Type::kObject) {
        for (const auto& [name, value] : weights->object) {
            const auto it = kWeights.find(name);
            if (it == kWeights.end() || !value.Number() || *value.Number() < 0 || *value.Number() > 10) continue;
            settings::WriteString((std::wstring(L"Weight.") + it->second).c_str(), std::to_wstring(*value.Number()));
        }
    }
    const json::Value* vocabulary = json->Get("vocabulary");
    settings::WriteString(kKoreanWords, Lines(vocabulary ? vocabulary->Get("korean") : nullptr, KoreanWord));
    settings::WriteString(kEnglishWords, Lines(vocabulary ? vocabulary->Get("english") : nullptr, EnglishWord));
    const json::Value* avoid = json->Get("avoid");
    settings::WriteString(kKoreanAvoid, AvoidLines(avoid, "korean", KoreanWord));
    settings::WriteString(kEnglishAvoid, AvoidLines(avoid, "english", EnglishWord));
    const json::Value* examples = json->Get("examples");
    if (examples && examples->type == json::Value::Type::kArray && !examples->array.empty()) {
        settings::WriteDword(L"LearnFromChoices", 1);
        jk_set_predictor(engine, Predictor());
        jk_set_learning(engine, 1);
        size_t n = 0;
        for (const json::Value& example : examples->array) {
            if (n++ == 2000) break;
            const json::Value* t = example.Get("text");
            if (!t || !t->String()) continue;
            const json::Value* k = example.Get("keys");
            const std::string keys = k && k->String() ? *k->String() : std::string();
            const json::Value* c = example.Get("count");
            const int count = std::clamp(c && c->Number() ? static_cast<int>(*c->Number()) : 1, 1, 20);
            for (int i = 0; i < count; ++i) {
                jk_learn_example(engine, reinterpret_cast<const std::uint8_t*>(t->String()->data()),
                                 t->String()->size(), reinterpret_cast<const std::uint8_t*>(keys.data()),
                                 keys.size());
            }
        }
    }
    log::Write(L"applied training.json");
}

void OpenFolder() {
    const std::wstring folder = Folder();
    if (folder.empty()) return;
    CreateDirectoryW(folder.c_str(), nullptr);
    ShellExecuteW(nullptr, L"open", folder.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
}

}  // namespace jk::training

#endif
