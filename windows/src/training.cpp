#include "training.h"

#ifdef JK_PREDICT

#include <windows.h>
#include <shellapi.h>
#include <shlobj.h>

#include <algorithm>
#include <atomic>
#include <map>
#include <mutex>
#include <string>

#include "json.h"
#include "log.h"
#include "settings.h"
#include "shared_data.h"
#include "training_lines.h"

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

std::string Narrow(const std::wstring& w) {
    if (w.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, w.data(), static_cast<int>(w.size()), nullptr, 0,
                                      nullptr, nullptr);
    std::string s(static_cast<size_t>(n), '\0');
    WideCharToMultiByte(CP_UTF8, 0, w.data(), static_cast<int>(w.size()), s.data(), n, nullptr, nullptr);
    return s;
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

namespace {

std::mutex g_mutex;
std::wstring g_words;  // the words `g_personal` was built with
JkPredictor* g_personal = nullptr;

}  // namespace

bool SetPredictor(JkEngine* engine) {
    const std::wstring korean = settings::ReadString(kKoreanWords).value_or(L"");
    const std::wstring english = settings::ReadString(kEnglishWords).value_or(L"");
    const std::wstring avoidKorean = settings::ReadString(kKoreanAvoid).value_or(L"");
    const std::wstring avoidEnglish = settings::ReadString(kEnglishAvoid).value_or(L"");
    if (korean.empty() && english.empty() && avoidKorean.empty() && avoidEnglish.empty()) {
        // The shared predictor lives as long as the DLL.
        const JkPredictor* shared = SharedPredictor();
        jk_set_predictor(engine, shared);
        return shared != nullptr;
    }
    const std::wstring key = korean + L'\0' + english + L'\0' + avoidKorean + L'\0' + avoidEnglish;
    // Set on the engine (which clones its reference) under the lock: another thread rebuilding
    // the personal predictor frees the old one, and mustn't do it in between.
    std::lock_guard<std::mutex> lock(g_mutex);
    JkPredictor*& personal = g_personal;
    std::wstring& words = g_words;
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
    const JkPredictor* predictor = personal ? personal : SharedPredictor();
    jk_set_predictor(engine, predictor);
    return predictor != nullptr;
}

void Release() {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (g_personal) jk_predictor_free(g_personal);
    g_personal = nullptr;
    g_words.clear();
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
            if (it == kWeights.end() || !value.Number() || !(*value.Number() >= 0 && *value.Number() <= 10)) continue;
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
        SetPredictor(engine);
        jk_set_learning(engine, 1);
        size_t n = 0;
        for (const json::Value& example : examples->array) {
            if (n++ == 2000) break;
            const json::Value* t = example.Get("text");
            if (!t || !t->String()) continue;
            const json::Value* k = example.Get("keys");
            const std::string keys = k && k->String() ? *k->String() : std::string();
            const json::Value* c = example.Get("count");
            const int count = c && c->Number() ? ClampCount(*c->Number(), 1, 20, 1) : 1;
            for (int i = 0; i < count; ++i) {
                jk_learn_example(engine, reinterpret_cast<const std::uint8_t*>(t->String()->data()),
                                 t->String()->size(), reinterpret_cast<const std::uint8_t*>(keys.data()),
                                 keys.size());
            }
        }
    }
    log::Write(L"applied training.json");
}

namespace {

std::atomic<bool> g_applying{false};
std::atomic<unsigned> g_generation{0};

}  // namespace

void ApplyInBackground() {
    if (g_applying.exchange(true)) return;  // one at a time per process
    const bool queued = RunInBackground([] {
        JkEngine* engine = jk_new();
        if (engine) {
            // As the UI engine would have it: the mode, the dictionary and what was learned so far.
            jk_set_mode(engine, static_cast<std::uint32_t>(settings::ReadDword(L"Mode", 0)));
            jk_set_dictionary(engine, SharedDictionary());
            if (auto learned = settings::ReadString(L"Learned")) {
                const std::string bytes = Narrow(*learned);
                jk_learner_import(engine, reinterpret_cast<const std::uint8_t*>(bytes.data()), bytes.size());
            }
            ApplyIfChanged(engine);
            if (jk_learner_changed(engine)) {
                const size_t length = jk_learner_export(engine);
                settings::WriteString(L"Learned", Wide(std::string(
                    reinterpret_cast<const char*>(jk_learner_ptr(engine)), length)));
            }
            jk_free(engine);
        }
        ++g_generation;
        g_applying = false;
    });
    if (!queued) g_applying = false;
}

unsigned Generation() { return g_generation.load(); }

void OpenFolder() {
    const std::wstring folder = Folder();
    if (folder.empty()) return;
    CreateDirectoryW(folder.c_str(), nullptr);
    ShellExecuteW(nullptr, L"open", folder.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
}

}  // namespace jk::training

#endif
