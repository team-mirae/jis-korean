#include "shared_data.h"

#include <windows.h>

#include <mutex>

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

}  // namespace

void Preload() {
    static std::once_flag once;
    std::call_once(once, [] {
        DllAddRef();  // the DLL stays loaded while the callback runs
        const BOOL queued = TrySubmitThreadpoolCallback(
            [](PTP_CALLBACK_INSTANCE, void*) {
                SharedDictionary();
#ifdef JK_PREDICT
                if (settings::ReadDword(L"Predictions", 1)) SharedPredictor();
#endif
                DllRelease();
            },
            nullptr, nullptr);
        if (!queued) DllRelease();
    });
}

const JkDict* SharedDictionary() {
    static std::once_flag once;
    static JkDict* dict = nullptr;
    std::call_once(once, [] {
        const ULONGLONG start = GetTickCount64();
        const Bytes tsv = Resource(IDR_HANJA);
        if (tsv.data) dict = jk_dict_new(tsv.data, tsv.size);
        if (!dict) log::Write(L"hanja dictionary unavailable");
        log::Write(L"hanja dictionary loaded in %llu ms", GetTickCount64() - start);
    });
    return dict;
}

#ifdef JK_PREDICT
const JkPredictor* SharedPredictor() {
    static std::once_flag once;
    static JkPredictor* predictor = nullptr;
    std::call_once(once, [] {
        const ULONGLONG start = GetTickCount64();
        const Bytes model = Resource(IDR_RANKER);
        const Bytes en = Resource(IDR_COMMON_EN);
        const Bytes ko = Resource(IDR_COMMON_KO);
        const Bytes nouns = Resource(IDR_NOUNS);
        if (model.data && en.data && ko.data) {
            predictor = jk_predictor_new(model.data, model.size, en.data, en.size, ko.data,
                                         ko.size, nouns.data, nouns.size);
        }
#ifdef JK_LEARNED
        // The learned ranker (spec §10.13); without its model the rules rank.
        const Bytes learned = Resource(IDR_LEARNED);
        if (predictor && learned.data) {
            if (JkPredictor* ranked = jk_predictor_with_learned(predictor, learned.data, learned.size)) {
                jk_predictor_free(predictor);
                predictor = ranked;
            } else {
                log::Write(L"learned ranker unavailable: the rules rank");
            }
        }
#endif
        if (!predictor) log::Write(L"predictor unavailable");
        log::Write(L"predictor loaded in %llu ms", GetTickCount64() - start);
    });
    return predictor;
}
#endif

}  // namespace jk
