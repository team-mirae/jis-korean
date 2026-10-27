#include "shared_data.h"

#include <windows.h>

#include <mutex>
#include <new>

#include "dll.h"
#include "settings.h"
#include "log.h"
#include "resource.h"

namespace jk {
namespace {

struct Bytes {
    const std::uint8_t* data = nullptr;
    size_t size = 0;
};

Bytes Resource(int id) {
    HRSRC info = FindResourceW(DllInstance(), MAKEINTRESOURCEW(id), RT_RCDATA);
    if (!info) return {};
    HGLOBAL handle = LoadResource(DllInstance(), info);
    if (!handle) return {};
    return {static_cast<const std::uint8_t*>(LockResource(handle)), SizeofResource(DllInstance(), info)};
}

std::once_flag g_dict_once;
JkDict* g_dict = nullptr;
#ifdef JK_PREDICT
std::once_flag g_predictor_once;
JkPredictor* g_predictor = nullptr;
#endif

}  // namespace

bool RunInBackground(void (*job)()) {
    // A reference on the DLL that the thread pool drops only after the callback has returned
    // (FreeLibraryWhenCallbackReturns): DllRelease alone, before returning through DLL code,
    // let the DLL unload under it.
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                            reinterpret_cast<LPCWSTR>(&RunInBackground), &module)) {
        return false;
    }
    struct Job {
        void (*run)();
        HMODULE module;
    };
    auto* context = new (std::nothrow) Job{job, module};
    DllAddRef();  // DllCanUnloadNow says no meanwhile
    const BOOL queued = context && TrySubmitThreadpoolCallback(
        [](PTP_CALLBACK_INSTANCE instance, void* context) {
            auto* job = static_cast<Job*>(context);
            const HMODULE module = job->module;
            job->run();
            delete job;
            DllRelease();
            FreeLibraryWhenCallbackReturns(instance, module);
        },
        context, nullptr);
    if (!queued) {
        delete context;
        DllRelease();
        FreeLibrary(module);
    }
    return queued != FALSE;
}

void Preload() {
#ifdef JK_PREDICT
    static std::once_flag once;
    std::call_once(once, [] {
        // With predictions on in this program the dictionary is part of predicting (hanja
        // readings, the learned ranker). Otherwise it's parsed at the first F7 (most programs
        // never see one): 50 MB of text, about 100 ms, not in every process.
        RunInBackground([] {
            if (settings::ReadDword(L"Predictions", 1) && settings::AppPredicting(settings::ExeName())) {
                SharedPredictor();
                SharedDictionary();
            }
        });
    });
#endif
}

const JkDict* SharedDictionary() {
    std::call_once(g_dict_once, [] {
        const ULONGLONG start = GetTickCount64();
        const Bytes tsv = Resource(IDR_HANJA);
        if (tsv.data) g_dict = jk_dict_new(tsv.data, tsv.size);
        if (!g_dict) log::Write(L"hanja dictionary unavailable");
        log::Write(L"hanja dictionary loaded in %llu ms", GetTickCount64() - start);
    });
    return g_dict;
}

#ifdef JK_PREDICT
const JkPredictor* SharedPredictor() {
    std::call_once(g_predictor_once, [] {
        const ULONGLONG start = GetTickCount64();
        const Bytes model = Resource(IDR_RANKER);
        const Bytes en = Resource(IDR_COMMON_EN);
        const Bytes ko = Resource(IDR_COMMON_KO);
        const Bytes nouns = Resource(IDR_NOUNS);
        if (model.data && en.data && ko.data) {
            g_predictor = jk_predictor_new(model.data, model.size, en.data, en.size, ko.data,
                                           ko.size, nouns.data, nouns.size);
        }
#ifdef JK_LEARNED
        // The learned ranker (spec §10.13); without its model the rules rank.
        const Bytes learned = Resource(IDR_LEARNED);
        if (g_predictor && learned.data) {
            if (JkPredictor* ranked = jk_predictor_with_learned(g_predictor, learned.data, learned.size)) {
                jk_predictor_free(g_predictor);
                g_predictor = ranked;
                log::Write(L"learned ranker loaded (%zu bytes)", learned.size);
            } else {
                log::Write(L"learned ranker unavailable: the rules rank");
            }
        }
#endif
        if (!g_predictor) log::Write(L"predictor unavailable");
        log::Write(L"predictor loaded in %llu ms", GetTickCount64() - start);
    });
    return g_predictor;
}
#endif

void ReleaseSharedData() {
    // DLL unload: no engine is left (DllCanUnloadNow) and no background job runs (it holds a
    // module reference until it has returned).
    if (g_dict) jk_dict_free(g_dict);
    g_dict = nullptr;
#ifdef JK_PREDICT
    if (g_predictor) jk_predictor_free(g_predictor);
    g_predictor = nullptr;
#endif
}

}  // namespace jk
