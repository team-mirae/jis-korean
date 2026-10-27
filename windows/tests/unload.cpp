// jk_unloadtest: loads jiskorean.dll, activates the text service on this thread (which preloads
// the shared data on the thread pool), deactivates and releases it, and unloads the DLL, again and
// again. The process's private memory must not grow with each round (DLL_PROCESS_DETACH frees the
// shared data), and nothing may crash while the preload runs during the unload.
//
//   jk_unloadtest [path\to\jiskorean.dll] [rounds]
#include <windows.h>
#include <msctf.h>
#include <psapi.h>

#include <cstdio>
#include <cwchar>
#include <string>

namespace {

constexpr GUID kClsid = {0xd10651ad, 0x51ea, 0x4dba, {0x8b, 0xff, 0x19, 0x05, 0x26, 0x1b, 0x26, 0xa1}};

SIZE_T PrivateBytes() {
    PROCESS_MEMORY_COUNTERS_EX counters = {sizeof(counters)};
    GetProcessMemoryInfo(GetCurrentProcess(), reinterpret_cast<PROCESS_MEMORY_COUNTERS*>(&counters),
                         sizeof(counters));
    return counters.PrivateUsage;
}

// One round: load, activate, deactivate, release, unload. False if something failed.
bool Round(const std::wstring& path, ITfThreadMgr* threads, TfClientId client, DWORD wait) {
    HMODULE dll = LoadLibraryW(path.c_str());
    if (!dll) return false;
    using GetClassObject = HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, void**);
    using CanUnloadNow = HRESULT(STDAPICALLTYPE*)();
    auto get = reinterpret_cast<GetClassObject>(GetProcAddress(dll, "DllGetClassObject"));
    auto can_unload = reinterpret_cast<CanUnloadNow>(GetProcAddress(dll, "DllCanUnloadNow"));
    IClassFactory* factory = nullptr;
    ITfTextInputProcessorEx* tip = nullptr;
    bool ok = get && can_unload && SUCCEEDED(get(kClsid, IID_PPV_ARGS(&factory))) &&
              SUCCEEDED(factory->CreateInstance(nullptr, IID_PPV_ARGS(&tip)));
    if (factory) factory->Release();
    if (ok) {
        ok = SUCCEEDED(tip->ActivateEx(threads, client, 0));
        Sleep(wait);  // 0: unload while the preload may still run
        tip->Deactivate();
        tip->Release();
    }
    // The preload holds its own reference: the DLL stays mapped until it has returned.
    if (can_unload && can_unload() != S_OK) {
        Sleep(1000);
        if (can_unload() != S_OK) {
            std::printf("DllCanUnloadNow still says no: not unloading\n");
            return false;
        }
    }
    FreeLibrary(dll);
    return ok;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    wchar_t self[MAX_PATH];
    GetModuleFileNameW(nullptr, self, MAX_PATH);
    std::wstring path = self;
    path = path.substr(0, path.find_last_of(L'\\') + 1) + L"jiskorean.dll";
    if (argc > 1) path = argv[1];
    const int rounds = argc > 2 ? _wtoi(argv[2]) : 8;
    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    ITfThreadMgr* threads = nullptr;
    TfClientId client = TF_CLIENTID_NULL;
    if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&threads))) ||
        FAILED(threads->Activate(&client))) {
        std::printf("no thread manager\n");
        return 2;
    }
    SIZE_T first = 0;
    int failures = 0;
    for (int i = 0; i < rounds; ++i) {
        if (!Round(path, threads, client, i % 2 ? 0 : 600)) ++failures;
        Sleep(1200);  // let a preload left running finish and the DLL unmap
        const SIZE_T now = PrivateBytes();
        if (i == 1) first = now;
        std::printf("round %d: %zu MB private\n", i + 1, now >> 20);
    }
    threads->Deactivate();
    threads->Release();
    CoUninitialize();
    // From the second round on (the first loads what stays: TSF's own state), growth means a leak.
    const SIZE_T last = PrivateBytes();
    const bool leaks = rounds > 3 && last > first + (40u << 20);
    std::printf("%s\n", failures ? "rounds failed" : leaks ? "leaks" : "no growth");
    return failures || leaks ? 1 : 0;
}
