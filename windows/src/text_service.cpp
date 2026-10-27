#include "text_service.h"

#include <InputScope.h>

#include <cwchar>
#include <new>

#include "display_attribute.h"
#include "dll.h"
#include "edit_session.h"
#include "guids.h"
#include "key_map.h"
#include "log.h"
#include "settings.h"
#include "shared_data.h"
#include "training.h"

using Microsoft::WRL::ComPtr;

namespace jk {
namespace {

constexpr unsigned kVkHangul = 0x15;  // 한/영 (VK_HANGUL); Right Alt under Korean keyboard type 1
constexpr unsigned kVkTab = 0x09;

// In a host that refuses a composition, or ends it right after each update, the unconfirmed text
// stays on screen as plain text and is replaced at every key, so syllables still compose (#6,
// option A; proposed for spec §8.4, not decided). False: each key commits there (loose jamo).
constexpr bool kKeepTextWithoutComposition = true;
// An app that ends the composition this soon after the shell's update did it by itself, not the
// user (measured: 15-31 ms).
constexpr ULONGLONG kOwnTermination = 100;

#ifdef JK_PREDICT
// spec §10.5, in JK_WEIGHT_* order (as macOS's Weight.<Scenario>).
constexpr const wchar_t* kWeightNames[] = {L"Korean", L"English", L"Implied", L"Mixed",
                                           L"TwoSet", L"KeepFinals", L"Hanja", L"ImpliedL"};
#endif

// The settings menu (lang_bar.h).
enum MenuId : UINT {
    kMenuEnglish = 1,
    kMenuDedicatedSymbols,
    kMenuPredictions,
    kMenuAppPredictions,
    kMenuLearning,
    kMenuResetLearned,
    kMenuTrainingFolder,
    kMenuAppOff,
    kMenuEnterCommits,
    kMenuEnglishDefault,  // ABC's layout: the Korean mode's keyboard's English
    kMenuEnglishWindows,  // ABC's layout: Windows' own (keys pass through)
    kMenuModeFirst = 100,     // + the mode's index in the engine's list (jk_mode_id_at)
    kMenuEnglishFirst = 300,  // + the index of an English mode, as ABC's layout
};

// Mode C (spec §3.6), the mode whose symbol keys are optional (Dedicated symbol keys).
constexpr unsigned kModeC = 2;

// A DWORD setting that isn't there (no mode id is this large: spec §3.6.1).
constexpr unsigned long kNotSet = 0xFFFFFFFE;

// A mode's mode name (spec §3.6.1), "A", "A_QWERTY"; empty for an unknown id.
std::string ModeCode(std::uint32_t id) {
    return std::string(reinterpret_cast<const char*>(jk_mode_code_ptr(id)), jk_mode_code_len(id));
}

// The mode whose mode name is `code`, or 0xFFFFFFFF (Windows' layout) if this build lacks it.
std::uint32_t ModeByCode(const std::string& code) {
    for (std::uint32_t i = 0; i < jk_mode_count(); ++i) {
        if (ModeCode(jk_mode_id_at(i)) == code) return jk_mode_id_at(i);
    }
    return 0xFFFFFFFF;
}

std::wstring Utf16(const std::uint8_t* data, size_t length) {
    if (!data || !length) return {};
    const int n = MultiByteToWideChar(CP_UTF8, 0, reinterpret_cast<const char*>(data),
                                      static_cast<int>(length), nullptr, 0);
    std::wstring s(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, reinterpret_cast<const char*>(data), static_cast<int>(length),
                        s.data(), n);
    return s;
}

std::string Utf8(const std::wstring& s) {
    if (s.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0,
                                      nullptr, nullptr);
    std::string out(static_cast<size_t>(n), '\0');
    WideCharToMultiByte(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), out.data(), n, nullptr,
                        nullptr);
    return out;
}

bool Down(int vk) { return (GetKeyState(vk) & 0x8000) != 0; }

std::uint32_t Modifiers() {
    std::uint32_t mods = 0;
    if (Down(VK_SHIFT)) mods |= JK_MOD_SHIFT;
    if (Down(VK_CONTROL)) mods |= JK_MOD_CTRL;
    if (Down(VK_MENU)) mods |= JK_MOD_ALT;
    if (Down(VK_LWIN) || Down(VK_RWIN)) mods |= JK_MOD_META;
    return mods;
}

constexpr std::uint32_t kShortcut = JK_MOD_CTRL | JK_MOD_ALT | JK_MOD_META;

std::uint32_t KeyOf(WPARAM wparam, LPARAM lparam) {
    return EngineKey(static_cast<unsigned>(wparam), static_cast<unsigned>((lparam >> 16) & 0xFF),
                     (lparam & (1 << 24)) != 0);
}

bool SameObject(IUnknown* a, IUnknown* b) {
    ComPtr<IUnknown> x, y;
    if (!a || !b || FAILED(a->QueryInterface(IID_PPV_ARGS(&x))) ||
        FAILED(b->QueryInterface(IID_PPV_ARGS(&y)))) {
        return false;
    }
    return x == y;
}

HRESULT GetCompartment(IUnknown* manager_owner, REFGUID guid, ComPtr<ITfCompartment>* out) {
    ComPtr<ITfCompartmentMgr> manager;
    HRESULT hr = manager_owner->QueryInterface(IID_PPV_ARGS(&manager));
    if (FAILED(hr)) return hr;
    return manager->GetCompartment(guid, &*out);
}

LONG ReadCompartment(ITfCompartment* compartment) {
    VARIANT v;
    VariantInit(&v);
    if (!compartment || FAILED(compartment->GetValue(&v)) || v.vt != VT_I4) return 0;
    return v.lVal;
}

void WriteCompartment(ITfCompartment* compartment, TfClientId client_id, LONG value) {
    if (!compartment) return;
    VARIANT v;
    v.vt = VT_I4;
    v.lVal = value;
    compartment->SetValue(client_id, &v);
}

}  // namespace

TextService::TextService() : engine_(jk_new()) { DllAddRef(); }

void TextService::EnsureData() {
    if (data_ready_) return;
    data_ready_ = true;
    jk_set_dictionary(engine_, SharedDictionary());  // waits only if Preload hasn't finished
    ApplySettings();                                 // the predictor, now that it's there
}

TextService::~TextService() {
    jk_free(engine_);
    DllRelease();
}

STDMETHODIMP TextService::QueryInterface(REFIID riid, void** object) {
    if (!object) return E_INVALIDARG;
    if (riid == IID_IUnknown || riid == IID_ITfTextInputProcessor ||
        riid == IID_ITfTextInputProcessorEx) {
        *object = static_cast<ITfTextInputProcessorEx*>(this);
    } else if (riid == IID_ITfKeyEventSink) {
        *object = static_cast<ITfKeyEventSink*>(this);
    } else if (riid == IID_ITfThreadMgrEventSink) {
        *object = static_cast<ITfThreadMgrEventSink*>(this);
    } else if (riid == IID_ITfActiveLanguageProfileNotifySink) {
        *object = static_cast<ITfActiveLanguageProfileNotifySink*>(this);
    } else if (riid == IID_ITfCompositionSink) {
        *object = static_cast<ITfCompositionSink*>(this);
    } else if (riid == IID_ITfCompartmentEventSink) {
        *object = static_cast<ITfCompartmentEventSink*>(this);
    } else if (riid == IID_ITfDisplayAttributeProvider) {
        *object = static_cast<ITfDisplayAttributeProvider*>(this);
    } else {
        *object = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) TextService::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) TextService::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (!refs) delete this;
    return refs;
}

STDMETHODIMP TextService::Activate(ITfThreadMgr* thread_mgr, TfClientId client_id) {
    return ActivateEx(thread_mgr, client_id, 0);
}

STDMETHODIMP TextService::ActivateEx(ITfThreadMgr* thread_mgr, TfClientId client_id, DWORD) {
    thread_mgr_ = thread_mgr;
    client_id_ = client_id;
    active_ = true;
    log::Write(L"ActivateEx tid=%lu", GetCurrentThreadId());

    ComPtr<ITfCategoryMgr> categories;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_PPV_ARGS(&categories)))) {
        categories->RegisterGUID(kDisplayAttributeInput, &input_attribute_);
        categories->RegisterGUID(kDisplayAttributeConverted, &converted_attribute_);
    }

    ComPtr<ITfKeystrokeMgr> keystrokes;
    if (SUCCEEDED(thread_mgr_.As(&keystrokes))) {
        const HRESULT hr = keystrokes->AdviseKeyEventSink(client_id_, this, TRUE);
        if (FAILED(hr)) log::Write(L"AdviseKeyEventSink failed %#lx", hr);
    }
    ComPtr<ITfSource> source;
    if (SUCCEEDED(thread_mgr_.As(&source))) {
        source->AdviseSink(IID_ITfThreadMgrEventSink, static_cast<ITfThreadMgrEventSink*>(this),
                           &thread_sink_cookie_);
        source->AdviseSink(IID_ITfActiveLanguageProfileNotifySink,
                           static_cast<ITfActiveLanguageProfileNotifySink*>(this),
                           &profile_sink_cookie_);
    }

    lang_bar_ = new (std::nothrow) LangBarButton(
        [this](ITfMenu* menu) { InitMenu(menu); }, [this](UINT id) { OnMenu(id); },
        [this] { WriteEnglish(open_); });
    ComPtr<ITfLangBarItemMgr> items;
    if (lang_bar_ && SUCCEEDED(thread_mgr_.As(&items))) items->AddItem(lang_bar_);

    // 한/영: this session's state, which every app shares.
    AdviseCompartments(true);
    open_ = !ReadEnglish();
    SetOpen(open_);
    if (lang_bar_) lang_bar_->SetState(mode_, open_, english_mode_);

    jk_activate(engine_);
    Preload();
    Activated();
    return S_OK;
}

void TextService::Activated() {
    ApplySettings();
#ifdef JK_PREDICT
    if (!training::Changed()) return;
    EnsureData();
    // A new or changed training file (docs/training-contract.md): applied, then settings again.
    training::ApplyIfChanged(engine_);
    SaveLearned();
    ApplySettings();
#endif
}

STDMETHODIMP TextService::Deactivate() {
    log::Write(L"Deactivate tid=%lu", GetCurrentThreadId());
    // Switching to another input method commits what's unconfirmed (spec §5.1).
    if (ITfContext* pending = Target(nullptr)) Flush(pending);
    // A flush TSF queued ends its composition when it runs, and starts nothing (EditText).
    active_ = false;
    refused_ = false;
    orphan_.Reset();
    orphan_context_.Reset();
    jk_clear(engine_);
    list_.Hide();
    AdviseCompartments(false);
    if (thread_mgr_) {
        ComPtr<ITfLangBarItemMgr> items;
        if (lang_bar_ && SUCCEEDED(thread_mgr_.As(&items))) items->RemoveItem(lang_bar_);
        ComPtr<ITfKeystrokeMgr> keystrokes;
        if (SUCCEEDED(thread_mgr_.As(&keystrokes))) keystrokes->UnadviseKeyEventSink(client_id_);
        ComPtr<ITfSource> source;
        if (SUCCEEDED(thread_mgr_.As(&source))) {
            if (thread_sink_cookie_ != TF_INVALID_COOKIE) source->UnadviseSink(thread_sink_cookie_);
            if (profile_sink_cookie_ != TF_INVALID_COOKIE) source->UnadviseSink(profile_sink_cookie_);
        }
    }
    if (lang_bar_) {
        lang_bar_->Detach();
        lang_bar_->Release();
        lang_bar_ = nullptr;
    }
    thread_sink_cookie_ = profile_sink_cookie_ = TF_INVALID_COOKIE;
    composition_.Reset();
    composition_context_.Reset();
    thread_mgr_.Reset();
    client_id_ = TF_CLIENTID_NULL;
    return S_OK;
}

// --- keys

bool TextService::Composing() const {
    if (jk_preedit_len(engine_) > 0 || jk_candidate_count(engine_) > 0) return true;
#ifdef JK_PREDICT
    if (shown_) return true;
#endif
    return false;
}

ITfContext* TextService::Target(ITfContext* context) const {
    if (context) return context;
    return composition_context_ ? composition_context_.Get() : orphan_context_.Get();
}

HRESULT TextService::NoContext(unsigned vk) {
    // No document to type into (a game window without an input context, a custom-drawn control):
    // the key is the app's, and nothing stays in the engine where it can't be seen (#3).
    static bool logged = false;
    if (!logged) log::Write(L"a key without an input context: passed on");
    logged = true;
    if (Composing()) DropInvisible();
    if (vk < eaten_down_.size()) eaten_down_[vk] = false;
    return S_OK;
}

void TextService::CheckOrphan() {
    if (!orphan_) return;
    // Plain text kept in place of a composition (#6) is only replaced while the caret is still
    // in or after it: moved away, what's shown stays as typed and the engine starts fresh.
    bool inside = true;
    ComPtr<ITfRange> orphan = orphan_;
    ComPtr<ITfContext> context = orphan_context_;
    // Gone from the document (an app that hands it on, as the IMM layer does): can't be replaced.
    bool gone = false;
    RunReadSessionNow(context.Get(), client_id_, [&](TfEditCookie ec) -> HRESULT {
        BOOL empty = FALSE;
        gone = SUCCEEDED(orphan->IsEmpty(ec, &empty)) && empty;
        inside = CaretInside(ec, context.Get(), orphan.Get());
        return S_OK;
    });
    if (gone) {
        static bool logged = false;
        if (!logged) log::Write(L"kept text left the document: the next key starts fresh");
        logged = true;
    }
    if (gone || !inside) DropInvisible();
}

void TextService::DropInvisible() {
    orphan_.Reset();
    orphan_context_.Reset();
    jk_clear(engine_);
#ifdef JK_PREDICT
    predictions_.clear();
    chosen_.reset();
    shown_.reset();
#endif
    list_.Hide();
}

bool TextService::PrivateField(ITfContext* context) {
    // A password-style edit control, however it's labelled: ES_PASSWORD, or a password character
    // set (EM_GETPASSWORDCHAR). Only Edit and RichEdit classes are asked: custom controls may
    // read the message as something else.
    if (HWND focus = GetFocus()) {
        wchar_t name[64] = L"";
        GetClassNameW(focus, name, 64);
        for (wchar_t& c : name) c = static_cast<wchar_t>(towlower(c));
        if (wcsstr(name, L"edit")) {
            if (GetWindowLongW(focus, GWL_STYLE) & ES_PASSWORD) return Private(L"ES_PASSWORD");
            if (SendMessageW(focus, EM_GETPASSWORDCHAR, 0, 0) != 0) return Private(L"password character");
        }
    }
    if (!context) return false;
    // The app turned the keyboard off for this field (password boxes through the IMM layer).
    for (const GUID* guid : {&GUID_COMPARTMENT_KEYBOARD_DISABLED, &GUID_COMPARTMENT_EMPTYCONTEXT}) {
        ComPtr<ITfCompartment> compartment;
        if (SUCCEEDED(GetCompartment(context, *guid, &compartment)) &&
            ReadCompartment(compartment.Get()) != 0) {
            return Private(guid == &GUID_COMPARTMENT_KEYBOARD_DISABLED ? L"keyboard disabled" : L"empty context");
        }
    }
    // A password or PIN by its input scope (browsers' password inputs, XAML PasswordBox...).
    bool secret = false;
    std::wstring scopes;
    RunReadSessionNow(context, client_id_, [&](TfEditCookie ec) -> HRESULT {
        ComPtr<ITfProperty> property;
        TF_SELECTION selection = {};
        ULONG fetched = 0;
        if (FAILED(context->GetProperty(kPropInputScope, &property)) ||
            FAILED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &fetched)) ||
            !fetched || !selection.range) {
            return S_OK;
        }
        VARIANT value;
        VariantInit(&value);
        ComPtr<ITfInputScope> input_scope;
        if (SUCCEEDED(property->GetValue(ec, selection.range, &value)) && value.vt == VT_UNKNOWN &&
            value.punkVal && SUCCEEDED(value.punkVal->QueryInterface(IID_PPV_ARGS(&input_scope)))) {
            InputScope* list = nullptr;
            UINT count = 0;
            if (SUCCEEDED(input_scope->GetInputScopes(&list, &count)) && list) {
                for (UINT i = 0; i < count; ++i) {
                    scopes += std::to_wstring(list[i]) + L" ";
                    switch (list[i]) {
                        case IS_PASSWORD: case IS_PRIVATE: case IS_NUMERIC_PASSWORD:
                        case IS_NUMERIC_PIN: case IS_ALPHANUMERIC_PIN: case IS_ALPHANUMERIC_PIN_SET:
                            secret = true;
                            break;
                        default:
                            break;
                    }
                }
                CoTaskMemFree(list);
            }
        }
        VariantClear(&value);
        selection.range->Release();
        return S_OK;
    });
    // Input scope numbers only (InputScope.h), never text: why a field was or wasn't private.
    if (scopes != last_scopes_) {
        log::Write(L"input scopes: %s", scopes.empty() ? L"none" : scopes.c_str());
        last_scopes_ = scopes;
    }
    return secret ? Private(L"input scope") : false;
}

bool TextService::Private(const wchar_t* why) {
    // Logged once per field: what made it private (never its text).
    if (!private_) log::Write(L"private field (%s): keys pass through", why);
    return true;
}

bool TextService::TypesEnglish(std::uint32_t key) const {
    // ABC, outside password fields and programs JIS Korean is off in (those pass everything).
    return !open_ && !private_ && !app_off_ && key != JK_KEY_MODIFIER && key != JK_KEY_OTHER;
}

bool TextService::Wants(unsigned vk, std::uint32_t key, std::uint32_t mods) const {
    // A password field: every key goes to the app untouched, as in ABC. Nothing is composed,
    // listed or learned there, so nothing of it can show on screen.
    if (private_ || app_off_ || !open_ || key == JK_KEY_MODIFIER) return false;
    // While composing every key goes through the engine: it commits before keys it doesn't use.
    if (Composing()) return true;
    if (key == JK_KEY_OTHER || (mods & kShortcut)) return false;
    // With nothing unconfirmed these change nothing in the engine: leave them to the app.
    switch (key) {
        case JK_KEY_ENTER: case JK_KEY_BACKSPACE: case JK_KEY_ESCAPE: case JK_KEY_ARROWUP:
        case JK_KEY_ARROWDOWN: case JK_KEY_ARROWLEFT: case JK_KEY_ARROWRIGHT:
            return false;
    }
    (void)vk;
    return true;
}

STDMETHODIMP TextService::OnSetFocus(BOOL foreground) {
    tested_ = {};  // a key-up may have gone elsewhere
    if (foreground) ApplySettings();
    return S_OK;
}

STDMETHODIMP TextService::OnTestKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam,
                                        BOOL* eaten) {
    // The engine runs here, not in OnKeyDown: a key reported as eaten here never reaches the app,
    // even if OnKeyDown then hands it back (measured: Enter, arrows and Tab after a commit were
    // lost in an Edit control and a RichEdit). So a key the engine doesn't consume is committed
    // before and reported as not eaten; the app then gets it as usual. OnKeyDown, which follows
    // for eaten keys, takes the result from here.
    const unsigned vk = static_cast<unsigned>(wparam);
    *eaten = FALSE;
    if (vk == kVkHangul) {
        *eaten = TRUE;  // handled in OnKeyDown
        return S_OK;
    }
    // The same key-down tested again (a message loop that peeks before it takes, an app calling
    // ITfKeystrokeMgr::TestKeyDown itself): the engine already ran for it (#5).
    const LONG time = GetMessageTime();
    const ULONGLONG tick = GetTickCount64();
    if (tested_.valid && SameKeyDown(tested_.wparam, tested_.lparam, tested_.time, tested_.tick,
                                     wparam, lparam, time, tick)) {
        static bool logged = false;
        if (!logged) log::Write(L"a key-down was tested twice: the first result stands");
        logged = true;
        *eaten = tested_.eaten;
        return S_OK;
    }
    tested_ = {};
    if (!Target(context)) return NoContext(vk);
    if (vk == vk::kPacket) {
        // Whether injected characters reach the TIP at all, and what lParam carries (#4).
        static bool logged = false;
        if (!logged) log::Write(L"VK_PACKET key-down, lParam %#llx", static_cast<unsigned long long>(lparam));
        logged = true;
    }
    const std::uint32_t key = KeyOf(wparam, lparam);
    const std::uint32_t mods = Modifiers();
    if (!Composing()) private_ = PrivateField(context);
    if (TypesEnglish(key)) {
        // ABC: the English layout's character, typed here for the same reason as below.
        *eaten = TypeEnglish(Target(context), key, mods);
        tested_ = {wparam, lparam, time, tick, *eaten, true, true};
    } else if (Wants(vk, key, mods)) {
        EnsureData();
        HandleKey(Target(context), vk, key, mods, eaten);
        tested_ = {wparam, lparam, time, tick, *eaten, true, true};
    }
    if (vk < eaten_down_.size()) eaten_down_[vk] = *eaten != FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam,
                                    BOOL* eaten) {
    const unsigned vk = static_cast<unsigned>(wparam);
    *eaten = FALSE;
    // The same key, already handled by OnTestKeyDown. Its lParam comes without the repeat count
    // here (measured), so compare the scan code and extended flag. Apps going through the IMM
    // layer call OnKeyDown even for keys OnTestKeyDown didn't eat (measured: Enter).
    constexpr LPARAM kScanAndExtended = 0x01FF0000;
    if (tested_.pending && tested_.wparam == wparam &&
        (tested_.lparam & kScanAndExtended) == (lparam & kScanAndExtended)) {
        tested_.pending = false;
        *eaten = tested_.eaten;
        return S_OK;
    }
    tested_ = {};
    context = Target(context);
    if (vk == kVkHangul) {
        // 한/영, as the Microsoft IME: commit, then switch every app (the session's state). It
        // needs no context: it only writes the session's compartment.
        if (open_ && Composing()) Flush(context);
        WriteEnglish(open_);
        *eaten = TRUE;
    } else if (!context) {
        return NoContext(vk);
    } else {
        const std::uint32_t key = KeyOf(wparam, lparam);
        const std::uint32_t mods = Modifiers();
        if (!Composing()) private_ = PrivateField(context);
        if (TypesEnglish(key)) {
            *eaten = TypeEnglish(context, key, mods);
        } else if (Wants(vk, key, mods)) {
            EnsureData();
            HandleKey(context, vk, key, mods, eaten);
        }
    }
    if (vk < eaten_down_.size()) eaten_down_[vk] = *eaten != FALSE;
    return S_OK;
}

void TextService::HandleKey(ITfContext* context, unsigned vk, std::uint32_t key,
                            std::uint32_t mods, BOOL* eaten) {
    Settle();  // a queued session's refusal, before the engine sees the next key
    CheckOrphan();
#ifdef JK_PREDICT
    if (HandleList(context, vk, key, mods)) {
        *eaten = TRUE;
        return;
    }
    // A key that commits takes the reading on screen, not the Korean one underneath, and then does
    // to it what it does to any text (Space adds a space, Shift+Enter only commits): the engine
    // does both (jk_key_after_prediction), as on macOS.
    const bool commits = (mods & kShortcut) || vk == kVkTab || key == JK_KEY_SPACE ||
                         key == JK_KEY_ENTER || key == JK_KEY_ESCAPE || key == JK_KEY_ARROWUP ||
                         key == JK_KEY_ARROWDOWN || key == JK_KEY_ARROWLEFT ||
                         key == JK_KEY_ARROWRIGHT || key == JK_KEY_OTHER;
    if (shown_ && commits) {
        *eaten = jk_key_after_prediction(engine_, chosen_.value_or(0), key, mods) != 0;
        chosen_.reset();
        shown_.reset();
        Apply(context);
        SaveLearned();
        return;
    }
    chosen_.reset();
#else
    (void)vk;
#endif
    *eaten = jk_key(engine_, key, mods) != 0;
    Apply(context);
#ifdef JK_PREDICT
    SaveLearned();
#endif
}

STDMETHODIMP TextService::OnTestKeyUp(ITfContext*, WPARAM wparam, LPARAM, BOOL* eaten) {
    // The key-down is over: a key-down of this key after this is a new press (#5).
    if (tested_.wparam == wparam) tested_ = {};
    *eaten = wparam < eaten_down_.size() && eaten_down_[wparam];
    return S_OK;
}

STDMETHODIMP TextService::OnKeyUp(ITfContext*, WPARAM wparam, LPARAM, BOOL* eaten) {
    if (tested_.wparam == wparam) tested_ = {};
    // The key-up of a key-down the app never saw stays away from it too.
    *eaten = wparam < eaten_down_.size() && eaten_down_[wparam];
    if (wparam < eaten_down_.size()) eaten_down_[wparam] = false;
    return S_OK;
}

STDMETHODIMP TextService::OnPreservedKey(ITfContext*, REFGUID, BOOL* eaten) {
    *eaten = FALSE;
    return S_OK;
}

// --- text out

void TextService::Apply(ITfContext* context) {
    const std::wstring commit = Utf16(jk_commit_ptr(engine_), jk_commit_len(engine_));
#ifdef JK_PREDICT
    predictions_.clear();
    const bool staged = jk_preedit_len(engine_) > 0;
    if (staged && predicting_ && jk_candidate_count(engine_) == 0) {
        const size_t count = jk_predict(engine_);
        for (size_t i = 0; i < count; ++i) {
            predictions_.push_back(Utf16(jk_prediction_ptr(engine_, i), jk_prediction_len(engine_, i)));
        }
        const bool top_is_korean = count == 0 || jk_prediction_kind(engine_, 0) == JK_PREDICTION_KOREAN;
        if (chosen_ && *chosen_ >= count) chosen_.reset();
        if (chosen_) {
            shown_ = predictions_[*chosen_];
        } else if (!top_is_korean) {
            shown_ = predictions_[0];
        } else {
            shown_.reset();
        }
    } else {
        chosen_.reset();
        shown_.reset();
    }
#endif
    Refresh(context, commit);
}

void TextService::Refresh(ITfContext* context, const std::wstring& commit) {
    std::wstring preedit = Utf16(jk_preedit_ptr(engine_), jk_preedit_len(engine_));
    bool converted = jk_candidate_count(engine_) > 0;
#ifdef JK_PREDICT
    if (chosen_ && *chosen_ < predictions_.size()) shown_ = predictions_[*chosen_];
    if (shown_) {
        preedit = *shown_;
        converted = true;
    }
#endif
    // The engine never holds text the user can't see (#3): unconfirmed text with no composition
    // to show it, once the update has run, is dropped.
    if (UpdateText(context, commit, preedit, converted) && Composing() && !composition_ && !orphan_) {
        static bool logged = false;
        if (!logged) log::Write(L"unconfirmed text with nothing showing it: dropped");
        logged = true;
        DropInvisible();
    }
    ShowList();
}

void TextService::ShowList() {
    const size_t hanja = jk_candidate_count(engine_);
    if (hanja > 0) {
        const size_t page = jk_candidate_page_size();
        // A selected index equal to the count: nothing selected yet (spec §3.6.3); the first page.
        const size_t raw = jk_candidate_selected(engine_);
        const bool none = raw >= hanja;
        const size_t selected = none ? 0 : raw;
        const size_t start = selected / page * page;
        std::vector<CandidateWindow::Row> rows;
        for (size_t i = start; i < hanja && i < start + page; ++i) {
            rows.push_back({std::to_wstring(i - start + 1),
                            Utf16(jk_candidate_ptr(engine_, i), jk_candidate_len(engine_, i)),
                            Utf16(jk_candidate_comment_ptr(engine_, i),
                                  jk_candidate_comment_len(engine_, i))});
        }
        list_.Show(std::move(rows), none ? -1 : static_cast<int>(selected - start),
                   (none ? L"" : std::to_wstring(selected + 1) + L"/") + std::to_wstring(hanja), anchor_,
                   [this, start](size_t row) { PickHanja(start + row); });
        return;
    }
#ifdef JK_PREDICT
    if (Composing() && predictions_.size() > 1) {
        std::vector<CandidateWindow::Row> rows;
        for (const std::wstring& p : predictions_) rows.push_back({{}, p, {}});
        list_.Show(std::move(rows), chosen_ ? static_cast<int>(*chosen_) : -1, {}, anchor_,
                   [this](size_t row) {
                       if (ITfContext* pending = Target(nullptr)) CommitPrediction(row, pending);
                   });
        return;
    }
#endif
    list_.Hide();
}

// A click on a hanja candidate: move the engine's selection there, then pick it.
void TextService::PickHanja(size_t index) {
    if (!Target(nullptr)) return;
    Settle();
    const size_t selected = jk_candidate_selected(engine_);
    const std::uint32_t key = index > selected ? JK_KEY_ARROWDOWN : JK_KEY_ARROWUP;
    for (size_t i = 0; i < (index > selected ? index - selected : selected - index); ++i) {
        jk_key(engine_, key, 0);
    }
    jk_key(engine_, JK_KEY_ENTER, 0);
    ComPtr<ITfContext> context = Target(nullptr);
    Apply(context.Get());
}

void TextService::Flush(ITfContext* context) {
    Settle();
#ifdef JK_PREDICT
    if (shown_) {
        // Committed because focus moved or the IME switched: not a choice to learn from.
        jk_flush_prediction(engine_, chosen_.value_or(0));
        chosen_.reset();
        shown_.reset();
        Apply(context);
        return;
    }
#endif
    jk_flush(engine_);
    Apply(context);
}

#ifdef JK_PREDICT
bool TextService::HandleList(ITfContext* context, unsigned vk, std::uint32_t key,
                             std::uint32_t mods) {
    if (predictions_.size() < 2 || jk_candidate_count(engine_) > 0 || (mods & kShortcut)) {
        return false;
    }
    const size_t count = predictions_.size();
    const bool shift = (mods & JK_MOD_SHIFT) != 0;
    if (key == JK_KEY_ARROWDOWN || (vk == kVkTab && !shift)) {
        chosen_ = chosen_ ? (*chosen_ + 1) % count : 0;
    } else if (key == JK_KEY_ARROWUP || (vk == kVkTab && shift)) {
        chosen_ = chosen_ ? (*chosen_ + count - 1) % count : count - 1;
    } else if (key == JK_KEY_ESCAPE && chosen_) {
        chosen_.reset();
        shown_.reset();
        Apply(context);  // the top reading again
        return true;
    } else if (key == JK_KEY_ENTER && chosen_) {
        CommitPrediction(*chosen_, context);
        return true;
    } else {
        return false;
    }
    Refresh(context);
    return true;
}

void TextService::CommitPrediction(size_t index, ITfContext* context) {
    jk_commit_prediction(engine_, index);
    chosen_.reset();
    shown_.reset();
    Apply(context);
    SaveLearned();
}

void TextService::SaveLearned() {
    if (!jk_learner_changed(engine_)) return;
    const size_t length = jk_learner_export(engine_);
    settings::WriteString(L"Learned", Utf16(jk_learner_ptr(engine_), length));
}
#endif

// --- settings

void TextService::ApplySettings() {
    app_off_ = settings::AppOff(settings::ExeName());
    // The mode follows the keyboard, not the app: one setting for every app (spec §8.4).
    SetMode(static_cast<unsigned>(settings::ReadDword(L"Mode", 0)));
    // ABC's layout (spec §8.4): the setting if it's an English mode this build has, or Windows'
    // own; else (unset, or a mode this build lacks) the Korean mode's keyboard's English.
    const unsigned long english = settings::ReadDword(L"EnglishMode", kNotSet);
    english_mode_ = english == kPassThrough || jk_mode_is_english(english) ? english : DefaultEnglish();
    if (lang_bar_) lang_bar_->SetState(mode_, open_, english_mode_);
    jk_set_dedicated_symbols(engine_, settings::ReadDword(L"DedicatedSymbols", 1) ? 1 : 0);
    jk_set_enter_commits(engine_, settings::ReadDword(L"EnterCommits", 0) ? 1 : 0);
#ifdef JK_PREDICT
    const bool on = settings::ReadDword(L"Predictions", 1) != 0 &&
                    settings::AppPredicting(settings::ExeName());
    // Before the first key the predictor may still be loading (EnsureData sets it then).
    const JkPredictor* predictor = on && data_ready_ ? training::Predictor() : nullptr;
    jk_set_predictor(engine_, predictor);
    predicting_ = predictor != nullptr;
    for (std::uint32_t id = 0; id < std::size(kWeightNames); ++id) {
        const std::wstring name = std::wstring(L"Weight.") + kWeightNames[id];
        if (auto value = settings::ReadString(name.c_str())) {
            jk_set_prediction_weight(engine_, id, wcstod(value->c_str(), nullptr));
        } else {
            jk_clear_prediction_weight(engine_, id);
        }
    }
    // The latest of what every engine learned: each saves after a change, each loads here.
    if (auto learned = settings::ReadString(L"Learned")) {
        const std::string bytes = Utf8(*learned);
        jk_learner_import(engine_, reinterpret_cast<const std::uint8_t*>(bytes.data()), bytes.size());
    } else {
        jk_learner_reset(engine_);
    }
    jk_set_learning(engine_, settings::ReadDword(L"LearnFromChoices", 0) ? 1 : 0);
#endif
}

std::uint32_t TextService::DefaultEnglish() const {
    // The same keyboard's QWERTY English (spec §3.6.2): JIS English on JIS, US English on US
    // ANSI, UK English on ISO.
    const std::string korean = ModeCode(mode_);
    const char* english = korean == "B" ? "B_QWERTY" : korean == "C" ? "CJ_QWERTY" : "A_QWERTY";
    return ModeByCode(english);
}

bool TextService::SameKeyboard(std::uint32_t english) const {
    // A mode name's namespace is its keyboard family (spec §3.6.1): A_ JIS, B_ US ANSI, C_ and
    // CJ_ ISO, the families of Modes A, B and C.
    const std::string korean = ModeCode(mode_), code = ModeCode(english);
    if (korean.empty() || code.empty()) return false;
    if (code.rfind(korean + "_", 0) == 0) return true;
    return korean == "C" && code.rfind("CJ_", 0) == 0;
}

bool TextService::TypeEnglish(ITfContext* context, std::uint32_t key, std::uint32_t mods) {
    // Shortcuts (Ctrl, Alt, Win) are the app's: Alt + letter variants (spec §3.6.3) are macOS's
    // only, since Alt + letter is Windows' menu key. Keys without a character (Space, Enter,
    // arrows, F-keys) go to the app too.
    if (english_mode_ == kPassThrough || (mods & kShortcut) || !context) return false;
    std::uint32_t ch = jk_legend_in(key, (mods & JK_MOD_SHIFT) ? 1 : 0, 0, english_mode_);
    if (!ch) return false;
    // Caps Lock types capitals (spec §3.6.2), and Shift then lower case, as Windows' layouts do.
    if ((GetKeyState(VK_CAPITAL) & 1) && ch < 0x80 && isalpha(static_cast<int>(ch))) {
        ch = isupper(static_cast<int>(ch)) ? static_cast<std::uint32_t>(tolower(static_cast<int>(ch)))
                                           : static_cast<std::uint32_t>(toupper(static_cast<int>(ch)));
    }
    std::wstring text;
    if (ch >= 0x10000) {
        text += static_cast<wchar_t>(0xD800 + ((ch - 0x10000) >> 10));
        text += static_cast<wchar_t>(0xDC00 + ((ch - 0x10000) & 0x3FF));
    } else {
        text += static_cast<wchar_t>(ch);
    }
    UpdateText(context, text, {}, false);
    return true;
}

void TextService::SetMode(unsigned mode) {
    if (mode == mode_) return;
    // An id this build doesn't have (a mode set by a newer version) is refused, and an English
    // mode isn't a Korean mode (it's ABC's layout, below): Mode A then.
    if (jk_mode_is_english(mode) || !jk_set_mode(engine_, mode)) {
        mode = 0;
        jk_set_mode(engine_, mode);
    }
    mode_ = mode;
    if (lang_bar_) lang_bar_->SetState(mode_, open_, english_mode_);
}

void TextService::InitMenu(ITfMenu* menu) {
    auto add = [menu](UINT id, const std::wstring& text, bool checked, bool enabled = true) {
        DWORD flags = checked ? TF_LBMENUF_CHECKED : 0;
        if (!enabled) flags |= TF_LBMENUF_GRAYED;
        menu->AddMenuItem(id, flags, nullptr, nullptr, text.c_str(),
                          static_cast<ULONG>(text.size()), nullptr);
    };
    auto separator = [menu] {
        menu->AddMenuItem(0, TF_LBMENUF_SEPARATOR, nullptr, nullptr, L"", 0, nullptr);
    };
    add(kMenuEnglish, L"ABC: English (한/영)", !open_);
    add(kMenuAppOff, L"Off in " + settings::ExeName() + L" (Keys Pass Through There)", app_off_);
    separator();
    // Every Korean mode this build has, in the engine's order and by its names (spec §3.6.1): ids
    // have gaps, so they're listed with jk_mode_id_at. "Mode A: JIS Korean Original Layout"...
    auto label = [](std::uint32_t id) {
        return Utf16(jk_mode_name_ptr(id), jk_mode_name_len(id)) + L" (" +
               Utf16(jk_mode_code_ptr(id), jk_mode_code_len(id)) + L")";
    };
    for (std::uint32_t index = 0; index < jk_mode_count(); ++index) {
        const std::uint32_t id = jk_mode_id_at(index);
        if (jk_mode_is_english(id)) continue;
        const std::wstring code = Utf16(jk_mode_code_ptr(id), jk_mode_code_len(id));
        const std::wstring name = Utf16(jk_mode_name_ptr(id), jk_mode_name_len(id));
        add(kMenuModeFirst + index, L"Mode " + code + L": " + name, mode_ == id);
    }
    // Mode C's symbol keys are optional (spec §3.6), as in macOS's input menu while it's active.
    if (mode_ == kModeC) {
        add(kMenuDedicatedSymbols, L"Dedicated Symbol Keys", settings::ReadDword(L"DedicatedSymbols", 1) != 0);
    }
    // What ABC (한/영) types (spec §8.4): by default the Korean mode's keyboard's English; any
    // English mode on request, this keyboard's first; or Windows' own layout.
    ITfMenu* english = nullptr;
    const std::wstring english_title = L"English Layout (한/영)";
    if (SUCCEEDED(menu->AddMenuItem(0, TF_LBMENUF_SUBMENU, nullptr, nullptr, english_title.c_str(),
                                    static_cast<ULONG>(english_title.size()), &english)) && english) {
        const bool unset = settings::ReadDword(L"EnglishMode", kNotSet) == kNotSet;
        auto item = [english](UINT id, const std::wstring& text, bool checked) {
            english->AddMenuItem(id, checked ? TF_LBMENUF_CHECKED : 0, nullptr, nullptr, text.c_str(),
                                 static_cast<ULONG>(text.size()), nullptr);
        };
        auto line = [english] { english->AddMenuItem(0, TF_LBMENUF_SEPARATOR, nullptr, nullptr, L"", 0, nullptr); };
        const std::uint32_t fallback = DefaultEnglish();
        item(kMenuEnglishDefault,
             L"Default for This Keyboard: " +
                 (fallback == kPassThrough ? std::wstring(L"Windows' Layout") : label(fallback)),
             unset);
        line();
        for (const bool same : {true, false}) {
            bool any = false;
            for (std::uint32_t index = 0; index < jk_mode_count(); ++index) {
                const std::uint32_t id = jk_mode_id_at(index);
                if (!jk_mode_is_english(id) || SameKeyboard(id) != same) continue;
                item(kMenuEnglishFirst + index, label(id), !unset && english_mode_ == id);
                any = true;
            }
            if (any) line();
        }
        item(kMenuEnglishWindows, L"Windows' Keyboard Layout (Keys Pass Through)",
             !unset && english_mode_ == kPassThrough);
        english->Release();
    }
    separator();
#ifdef JK_PREDICT
    add(kMenuPredictions, L"Predictions", settings::ReadDword(L"Predictions", 1) != 0);
    const std::wstring& exe = settings::ExeName();
    std::wstring app = L"Predictions in " + exe;
    if (const wchar_t* why = settings::DefaultOffReason(exe)) {
        app += std::wstring(L" (off by default: ") + why + L")";
    }
    add(kMenuAppPredictions, app, settings::AppPredicting(exe));
    add(kMenuLearning, L"Learn from My Choices (Stays on This PC)",
        settings::ReadDword(L"LearnFromChoices", 0) != 0);
    // What was learned, read-only, as on macOS.
    ITfMenu* learned = nullptr;
    const std::wstring title = L"Learned Weights";
    if (SUCCEEDED(menu->AddMenuItem(0, TF_LBMENUF_SUBMENU, nullptr, nullptr, title.c_str(),
                                    static_cast<ULONG>(title.size()), &learned)) && learned) {
        const wchar_t* labels[] = {L"Korean", L"English", L"Implied ㅇ", L"English + particle",
                                   L"Plain 2-set", L"Finals kept", L"Hanja", L"Implied ㄹ after ㄴ"};
        for (std::uint32_t id = 0; id < std::size(labels); ++id) {
            wchar_t line[64];
            swprintf_s(line, L"%s  ×%.2f", labels[id], jk_learned_weight(engine_, id));
            learned->AddMenuItem(0, TF_LBMENUF_GRAYED, nullptr, nullptr, line,
                                 static_cast<ULONG>(wcslen(line)), nullptr);
        }
        learned->AddMenuItem(0, TF_LBMENUF_SEPARATOR, nullptr, nullptr, L"", 0, nullptr);
        const std::wstring reset = L"Reset What It Learned", folder = L"Open Training Folder";
        learned->AddMenuItem(kMenuResetLearned, 0, nullptr, nullptr, reset.c_str(),
                             static_cast<ULONG>(reset.size()), nullptr);
        learned->AddMenuItem(kMenuTrainingFolder, 0, nullptr, nullptr, folder.c_str(),
                             static_cast<ULONG>(folder.size()), nullptr);
        learned->Release();
    }
    separator();
#endif
    add(kMenuEnterCommits, L"Enter Commits Without a New Line",
        settings::ReadDword(L"EnterCommits", 0) != 0);
}

void TextService::OnMenu(UINT id) {
    if (id >= kMenuModeFirst && id < kMenuModeFirst + jk_mode_count()) {
        if (ITfContext* pending = Target(nullptr)) Flush(pending);
        settings::WriteDword(L"Mode", jk_mode_id_at(id - kMenuModeFirst));
        ApplySettings();
        return;
    }
    if (id >= kMenuEnglishFirst && id < kMenuEnglishFirst + jk_mode_count()) {
        settings::WriteDword(L"EnglishMode", jk_mode_id_at(id - kMenuEnglishFirst));
        ApplySettings();
        return;
    }
    switch (id) {
        case kMenuEnglish:
            WriteEnglish(open_);
            return;
        case kMenuAppOff:
            if (ITfContext* pending = Target(nullptr); !app_off_ && pending) Flush(pending);
            settings::SetAppOff(settings::ExeName(), !app_off_);
            break;
        case kMenuEnglishDefault:
            settings::Delete(L"EnglishMode");
            break;
        case kMenuEnglishWindows:
            settings::WriteDword(L"EnglishMode", kPassThrough);
            break;
        case kMenuDedicatedSymbols:
            settings::WriteDword(L"DedicatedSymbols", settings::ReadDword(L"DedicatedSymbols", 1) ? 0 : 1);
            break;
        case kMenuEnterCommits:
            settings::WriteDword(L"EnterCommits", settings::ReadDword(L"EnterCommits", 0) ? 0 : 1);
            break;
#ifdef JK_PREDICT
        case kMenuPredictions:
            settings::WriteDword(L"Predictions", settings::ReadDword(L"Predictions", 1) ? 0 : 1);
            break;
        case kMenuAppPredictions: {
            const std::wstring& exe = settings::ExeName();
            settings::SetAppPredicting(exe, !settings::AppPredicting(exe));
            break;
        }
        case kMenuLearning:
            settings::WriteDword(L"LearnFromChoices",
                                 settings::ReadDword(L"LearnFromChoices", 0) ? 0 : 1);
            break;
        case kMenuTrainingFolder:
            training::OpenFolder();
            return;
        case kMenuResetLearned:
            jk_learner_reset(engine_);
            settings::Delete(L"Learned");
            break;
#endif
    }
    ApplySettings();
}

// --- 한/영

bool TextService::ReadEnglish() {
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> compartment;
    if (!thread_mgr_ || FAILED(thread_mgr_->GetGlobalCompartment(&global)) ||
        FAILED(global->GetCompartment(kCompartmentEnglish, &compartment))) {
        return false;
    }
    return ReadCompartment(compartment.Get()) != 0;
}

void TextService::WriteEnglish(bool english) {
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> compartment;
    if (!thread_mgr_ || FAILED(thread_mgr_->GetGlobalCompartment(&global)) ||
        FAILED(global->GetCompartment(kCompartmentEnglish, &compartment))) {
        SetOpen(!english);  // no global compartment: this thread at least
        return;
    }
    WriteCompartment(compartment.Get(), client_id_, english ? 1 : 0);
    // The change notification (OnChange) reaches this thread too; apply it now in any case.
    SetOpen(!english);
}

void TextService::SetOpen(bool open) {
    const bool changed = open != open_;
    if (ITfContext* pending = Target(nullptr); !open && changed && pending) Flush(pending);
    open_ = open;
    // The thread's open/close compartment is what the input indicator and apps read.
    ComPtr<ITfCompartment> openclose;
    if (thread_mgr_ &&
        SUCCEEDED(GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose)) &&
        (ReadCompartment(openclose.Get()) != 0) != open) {
        WriteCompartment(openclose.Get(), client_id_, open ? 1 : 0);
    }
    if (lang_bar_) lang_bar_->SetState(mode_, open_, english_mode_);
    if (changed) log::Write(L"%s", open ? L"한글" : L"ABC");
}

void TextService::AdviseCompartments(bool on) {
    if (!thread_mgr_) return;
    ComPtr<ITfCompartmentMgr> global;
    ComPtr<ITfCompartment> english, openclose;
    if (SUCCEEDED(thread_mgr_->GetGlobalCompartment(&global))) {
        global->GetCompartment(kCompartmentEnglish, &english);
    }
    GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose);
    struct Pair {
        ITfCompartment* compartment;
        DWORD* cookie;
    };
    for (Pair p : {Pair{english.Get(), &english_sink_cookie_},
                   Pair{openclose.Get(), &openclose_sink_cookie_}}) {
        ComPtr<ITfSource> source;
        if (!p.compartment || FAILED(p.compartment->QueryInterface(IID_PPV_ARGS(&source)))) continue;
        if (on && *p.cookie == TF_INVALID_COOKIE) {
            source->AdviseSink(IID_ITfCompartmentEventSink,
                               static_cast<ITfCompartmentEventSink*>(this), p.cookie);
        } else if (!on && *p.cookie != TF_INVALID_COOKIE) {
            source->UnadviseSink(*p.cookie);
            *p.cookie = TF_INVALID_COOKIE;
        }
    }
}

STDMETHODIMP TextService::OnChange(REFGUID guid) {
    if (IsEqualGUID(guid, kCompartmentEnglish)) {
        // Another app (or this one) pressed 한/영.
        SetOpen(!ReadEnglish());
    } else if (IsEqualGUID(guid, GUID_COMPARTMENT_KEYBOARD_OPENCLOSE) && thread_mgr_) {
        // The open/close compartment only mirrors the session's state. Windows closes it when a
        // Korean profile activates (measured: right after ActivateEx), so a change from elsewhere
        // isn't a 한/영 press: put it back. 한/영, the indicator's button and the menu switch.
        ComPtr<ITfCompartment> openclose;
        if (SUCCEEDED(GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose)) &&
            (ReadCompartment(openclose.Get()) != 0) != open_) {
            // Logged so apps that close the IME for a field (ImmSetOpenStatus, e.g. password
            // boxes in banking and certificate programs) show up; kept 한글 / ABC as it was.
            log::Write(L"open/close set to %s from outside; put back to %s", open_ ? L"closed" : L"open",
                       open_ ? L"open" : L"closed");
            WriteCompartment(openclose.Get(), client_id_, open_ ? 1 : 0);
        }
    }
    return S_OK;
}

// --- focus and profiles

STDMETHODIMP TextService::OnSetFocus(ITfDocumentMgr*, ITfDocumentMgr*) {
    // Focus moved to another document: what was unconfirmed is committed where it was (spec §5.1).
    tested_ = {};
    if (ITfContext* pending = Target(nullptr)) {
        Flush(pending);
    } else if (Composing()) {
        DropInvisible();  // text that never reached a document doesn't follow focus (#3)
    }
    list_.Hide();
    ApplySettings();  // a setting changed in another app applies here from now on
    return S_OK;
}

STDMETHODIMP TextService::OnActivated(REFCLSID clsid, REFGUID profile, BOOL activated) {
    if (!IsEqualGUID(clsid, kClsidTextService)) return S_OK;
    (void)profile;
    if (activated) {
        jk_activate(engine_);  // nothing to reset since the F8 escape was retired (spec §6.1)
        Activated();
    } else if (ITfContext* pending = Target(nullptr)) {
        Flush(pending);
    }
    return S_OK;
}

STDMETHODIMP TextService::OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) {
    // The app ended the composition (a click elsewhere, ...): its text stays as it is shown, and
    // the engine starts fresh.
    if (composition_ && composition == composition_.Get()) {
        // How soon after the shell's own update apps end it (#6): logged once per program.
        const ULONGLONG since = GetTickCount64() - updated_tick_;
        static bool logged = false;
        if (!logged) {
            log::Write(L"composition terminated %s%llu ms after update", in_update_ ? L"during the update, " : L"",
                       since);
        }
        logged = true;
        ComPtr<ITfRange> range;
        if (FAILED(composition->GetRange(&range))) range.Reset();
        if (range && composition_context_ && CaretInside(ec, composition_context_.Get(), range.Get())) {
            CaretToEnd(ec, composition_context_.Get(), range.Get());
        }
        // Ended by the app right after the shell's own update, not by the user (#6, option A): the
        // text stays as plain text, the engine keeps its segment, and the next update replaces it.
        // Not through the IMM layer (a transitory context): the ended composition's text leaves
        // the document there, as the app's result string (measured), so it can't be replaced.
        TF_STATUS status = {};
        const bool transitory = composition_context_ && SUCCEEDED(composition_context_->GetStatus(&status)) &&
                                (status.dwStaticFlags & TF_SS_TRANSITORY) != 0;
        if (kKeepTextWithoutComposition && range && !transitory && Composing() &&
            (in_update_ || since < kOwnTermination)) {
            ComPtr<ITfRange> orphan;
            if (SUCCEEDED(range->Clone(&orphan)) && orphan) {
                // Text typed right before or after it isn't part of it.
                orphan->SetGravity(ec, TF_GRAVITY_FORWARD, TF_GRAVITY_BACKWARD);
                orphan_ = orphan;
                orphan_context_ = composition_context_;
                composition_.Reset();
                composition_context_.Reset();
                return S_OK;
            }
        }
        composition_.Reset();
        composition_context_.Reset();
        DropInvisible();
    }
    return S_OK;
}

STDMETHODIMP TextService::EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** infos) {
    if (!infos) return E_INVALIDARG;
    *infos = NewDisplayAttributeEnum();
    return *infos ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP TextService::GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo** info) {
    if (!info) return E_INVALIDARG;
    *info = NewDisplayAttributeInfo(guid);
    return *info ? S_OK : E_INVALIDARG;
}

// --- the composition

bool TextService::UpdateText(ITfContext* context, const std::wstring& commit,
                             const std::wstring& preedit, bool converted) {
    if (!context) return true;
    // A composition left in another document (focus moved) ends there as it is.
    if (composition_ && composition_context_ && !SameObject(composition_context_.Get(), context)) {
        ComPtr<ITfComposition> old = composition_;
        RunEditSession(composition_context_.Get(), client_id_,
                       [old](TfEditCookie ec) { return old->EndComposition(ec); });
        composition_.Reset();
        composition_context_.Reset();
    }
    // Plain text kept for another document stays there as it is.
    if (orphan_ && !SameObject(orphan_context_.Get(), context)) {
        orphan_.Reset();
        orphan_context_.Reset();
    }
    if (!composition_ && !orphan_ && commit.empty() && preedit.empty()) return true;
    // When TSF can't run the session at once it runs after this returns, maybe after Deactivate:
    // it holds the service, the context and the composition it was meant for, and copies of the
    // text. It never calls the engine (Settle does, here, once the session has run).
    ComPtr<TextService> self(this);
    ComPtr<ITfContext> context_ref = context;
    ComPtr<ITfComposition> expected = composition_;
    const TfGuidAtom attribute = converted ? converted_attribute_ : input_attribute_;
    auto edit = [self, context_ref, expected, commit, preedit, attribute](TfEditCookie ec) -> HRESULT {
        return self->EditText(ec, context_ref.Get(), expected.Get(), commit, preedit, attribute);
    };
    bool async = false;
    in_update_ = true;
    const HRESULT hr = RunEditSession(context, client_id_, edit, &async);
    in_update_ = false;
    updated_tick_ = GetTickCount64();
    if (FAILED(hr)) log::Write(L"edit session failed %#lx", hr);
    if (async) {
        // The key may reach the app before this text does (Chromium holds the lock while it
        // handles a key): logged once per program, to measure before re-sending keys (#7).
        static bool logged = false;
        if (!logged) log::Write(L"edit session queued (async): the text lands after the key");
        logged = true;
        return false;
    }
    Settle();
    return true;
}

void TextService::Settle() {
    if (!refused_) return;
    refused_ = false;
    // The preedit went in as plain text: the engine lets go of it, committing what was shown.
#ifdef JK_PREDICT
    if (shown_) {
        jk_flush_prediction(engine_, chosen_.value_or(0));
    } else {
        jk_flush(engine_);
    }
    predictions_.clear();
    chosen_.reset();
    shown_.reset();
#else
    jk_flush(engine_);
#endif
    list_.Hide();
}

HRESULT TextService::EditText(TfEditCookie ec, ITfContext* context, ITfComposition* expected,
                              const std::wstring& commit, const std::wstring& preedit,
                              TfGuidAtom attribute_atom) {
    // Deactivated since this was requested: end what's shown with its commit, start nothing.
    if (!active_) {
        ComPtr<ITfRange> range;
        if (expected && SUCCEEDED(expected->GetRange(&range)) && range && !commit.empty()) {
            range->SetText(ec, 0, commit.data(), static_cast<LONG>(commit.size()));
        }
        if (expected) expected->EndComposition(ec);
        return S_OK;
    }
    // The composition this was meant for ended in between (the app ended it before a queued
    // session ran): its text stays as shown, and nothing is inserted next to it.
    if (expected && composition_.Get() != expected) {
        static bool logged = false;
        if (!logged) log::Write(L"a queued update found its composition ended: left as shown");
        logged = true;
        return S_OK;
    }
    ComPtr<ITfProperty> attribute;
    if (FAILED(context->GetProperty(GUID_PROP_ATTRIBUTE, &attribute))) attribute.Reset();
    // Apps that reach TSF through the IMM compatibility layer (classic Win32 controls; the
    // context is "transitory") end a Korean composition with a plain caret at once, like any
    // non-interim selection (FKeyIME docs/issues/korean-jamo-not-composed.md). Like the
    // Microsoft Korean IME, the composition is selected as an interim character there. TSF
    // only sets that on one character (longer: E_INVALIDARG), but keeps it as the
    // composition grows: after SetText the interim selection covers the whole composition
    // (measured). Interim on the last character only would make the IMM layer commit
    // everything before it, again at every key (measured: 한한한한글).
    TF_STATUS status = {};
    const bool transitory = SUCCEEDED(context->GetStatus(&status)) &&
                            (status.dwStaticFlags & TF_SS_TRANSITORY) != 0;
    auto show = [&](ITfRange* range) -> HRESULT {
        if (!transitory) return CaretToEnd(ec, context, range);
        TF_SELECTION selection = {};
        selection.range = range;
        selection.style.ase = TF_AE_NONE;
        selection.style.fInterimChar = TRUE;
        if (SUCCEEDED(context->SetSelection(ec, 1, &selection))) return S_OK;
        // The interim selection TSF kept from the first character, now over all of it?
        TF_SELECTION now = {};
        ULONG fetched = 0;
        bool kept = false;
        if (SUCCEEDED(context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &now, &fetched)) &&
            fetched && now.range) {
            LONG start = 1, end = 1;
            kept = now.style.fInterimChar &&
                   SUCCEEDED(now.range->CompareStart(ec, range, TF_ANCHOR_START, &start)) &&
                   SUCCEEDED(now.range->CompareEnd(ec, range, TF_ANCHOR_END, &end)) &&
                   start == 0 && end == 0;
            now.range->Release();
        }
        if (kept) return S_OK;
        // Once per process: some apps (Chromium-based ones, measured in Claude) never keep one.
        static bool logged = false;
        if (!logged) log::Write(L"no interim selection over the composition: caret after it");
        logged = true;
        return CaretToEnd(ec, context, range);
    };
    auto place = [&](ITfRange* range) {
        // Where the list goes: under the composition, else under the caret.
        RECT rect = {};
        BOOL clipped = FALSE;
        ComPtr<ITfContextView> view;
        if (FAILED(context->GetActiveView(&view)) ||
            FAILED(view->GetTextExt(ec, range, &rect, &clipped)) || IsRectEmpty(&rect)) {
            GUITHREADINFO gui = {sizeof(gui)};
            if (GetGUIThreadInfo(GetCurrentThreadId(), &gui) && gui.hwndCaret) {
                rect = gui.rcCaret;
                MapWindowPoints(gui.hwndCaret, nullptr, reinterpret_cast<POINT*>(&rect), 2);
            }
        }
        anchor_ = rect;
    };
    auto mark = [&](ITfRange* range) {
        if (attribute && attribute_atom != TF_INVALID_GUIDATOM) {
            VARIANT v;
            v.vt = VT_I4;
            v.lVal = static_cast<LONG>(attribute_atom);
            attribute->SetValue(ec, range, &v);
        }
        place(range);
    };
    auto set_text = [&](ITfRange* range, const std::wstring& text) {
        const HRESULT hr = range->SetText(ec, 0, text.data(), static_cast<LONG>(text.size()));
        if (FAILED(hr)) log::Write(L"SetText on the composition failed %#lx", hr);
        return hr;
    };

    ComPtr<ITfInsertAtSelection> insert;
    ComPtr<ITfContextComposition> compositions;
    if (FAILED(context->QueryInterface(IID_PPV_ARGS(&insert))) ||
        FAILED(context->QueryInterface(IID_PPV_ARGS(&compositions)))) {
        insert.Reset();
        compositions.Reset();
    }
    // A composition over `over`, showing the preedit. False: the app refused it.
    auto compose = [&](ITfRange* over) -> bool {
        ComPtr<ITfComposition> started;
        ComPtr<ITfRange> range;
        HRESULT hr = compositions ? compositions->StartComposition(ec, over, this, &started) : E_NOINTERFACE;
        if (SUCCEEDED(hr) && started && (FAILED(started->GetRange(&range)) || !range)) {
            started->EndComposition(ec);
            started.Reset();
            hr = E_FAIL;
        }
        if (FAILED(hr) || !started) {
            static bool logged = false;
            if (!logged) log::Write(L"composition refused hr=%#lx", hr);
            logged = true;
            return false;
        }
        composition_ = started;
        composition_context_ = context;
        set_text(range.Get(), preedit);
        mark(range.Get());
        show(range.Get());
        return true;
    };

    if (orphan_) {
        // Plain text shown in place of a composition the app refused or ended (#6, option A):
        // this update replaces it.
        ComPtr<ITfRange> orphan = orphan_;
        orphan_.Reset();
        orphan_context_.Reset();
        if (!commit.empty() || preedit.empty()) {
            set_text(orphan.Get(), commit);  // final now (possibly nothing)
            CaretToEnd(ec, context, orphan.Get());
        } else {
            // Still unconfirmed: a composition over it if the app takes one now, else plain text.
            if (compose(orphan.Get())) return S_OK;
            if (FAILED(set_text(orphan.Get(), preedit))) {
                refused_ = true;  // can't even replace it: Settle commits what's there
                return E_FAIL;
            }
            orphan_ = orphan;
            orphan_context_ = context;
            place(orphan.Get());
            return CaretToEnd(ec, context, orphan.Get());
        }
    } else if (composition_) {
        ComPtr<ITfRange> range;
        if (FAILED(composition_->GetRange(&range)) || !range) {
            // Ended by the app without telling us: its text stays as shown, and what follows
            // starts a new composition. The commit isn't inserted again next to it.
            log::Write(L"the composition has no range: left as shown");
            composition_.Reset();
            composition_context_.Reset();
        } else if (commit.empty() && !preedit.empty()) {  // still composing: just update it
            set_text(range.Get(), preedit);
            mark(range.Get());
            return show(range.Get());
        } else {
            // The composition ends; its text becomes `commit` (possibly nothing). The caret goes
            // after it only if it's still inside: a key the app already handled (a queued
            // session) may have moved it, and moving it back would undo that arrow or Tab.
            const bool inside = CaretInside(ec, context, range.Get());
            if (attribute) attribute->Clear(ec, range.Get());
            set_text(range.Get(), commit);
            if (inside) CaretToEnd(ec, context, range.Get());
            composition_->EndComposition(ec);
            composition_.Reset();
            composition_context_.Reset();
        }
    } else if (!commit.empty()) {
        InsertAtSelection(ec, context, commit);
    }
    if (preedit.empty()) return S_OK;

    ComPtr<ITfRange> at;
    if (!insert || FAILED(insert->InsertTextAtSelection(ec, TF_IAS_QUERYONLY, nullptr, 0, &at)) || !at) {
        refused_ = true;  // nothing shown: Settle lets the engine go of it
        return E_FAIL;
    }
    if (compose(at.Get())) return S_OK;
    // The app refused a composition: the preedit goes in as plain text.
    ComPtr<ITfRange> plain;
    if (kKeepTextWithoutComposition &&
        SUCCEEDED(insert->InsertTextAtSelection(ec, 0, preedit.data(), static_cast<LONG>(preedit.size()), &plain)) &&
        plain) {
        // Kept, and replaced at the next update; the engine keeps its segment (#6, option A).
        plain->SetGravity(ec, TF_GRAVITY_FORWARD, TF_GRAVITY_BACKWARD);  // nothing typed next to it joins it
        orphan_ = plain;
        orphan_context_ = context;
        place(plain.Get());
        return CaretToEnd(ec, context, plain.Get());
    }
    // Committed: Settle (after the session, outside it) commits it in the engine too.
    refused_ = true;
    return plain ? CaretToEnd(ec, context, plain.Get()) : InsertAtSelection(ec, context, preedit);
}

}  // namespace jk
