#include "text_service.h"

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

#ifdef JK_PREDICT
// spec §10.5, in JK_WEIGHT_* order (as macOS's Weight.<Scenario>).
constexpr const wchar_t* kWeightNames[] = {L"Korean", L"English", L"Implied", L"Mixed",
                                           L"TwoSet", L"KeepFinals", L"Hanja", L"ImpliedL"};
#endif

// The settings menu (lang_bar.h).
enum MenuId : UINT {
    kMenuEnglish = 1,
    kMenuModeA,
    kMenuModeB,
    kMenuPredictions,
    kMenuAppPredictions,
    kMenuLearning,
    kMenuResetLearned,
    kMenuTrainingFolder,
    kMenuEnterCommits,
};

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
    if (lang_bar_) lang_bar_->SetState(mode_, open_);

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
    if (composition_context_) Flush(composition_context_.Get());
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

bool TextService::Wants(unsigned vk, std::uint32_t key, std::uint32_t mods) const {
    if (!open_ || key == JK_KEY_MODIFIER) return false;
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
    const std::uint32_t key = KeyOf(wparam, lparam);
    const std::uint32_t mods = Modifiers();
    if (Wants(vk, key, mods)) {
        EnsureData();
        HandleKey(context ? context : composition_context_.Get(), vk, key, mods, eaten);
        tested_ = {wparam, lparam, GetTickCount64(), *eaten != FALSE, true};
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
        (tested_.lparam & kScanAndExtended) == (lparam & kScanAndExtended) &&
        GetTickCount64() - tested_.tick < 2000) {
        tested_.pending = false;
        *eaten = tested_.eaten;
        return S_OK;
    }
    tested_.pending = false;
    if (!context) context = composition_context_.Get();
    if (vk == kVkHangul) {
        // 한/영, as the Microsoft IME: commit, then switch every app (the session's state).
        if (open_ && Composing()) Flush(context);
        WriteEnglish(open_);
        *eaten = TRUE;
    } else {
        const std::uint32_t key = KeyOf(wparam, lparam);
        const std::uint32_t mods = Modifiers();
        if (Wants(vk, key, mods)) {
            EnsureData();
            HandleKey(context, vk, key, mods, eaten);
        }
    }
    if (vk < eaten_down_.size()) eaten_down_[vk] = *eaten != FALSE;
    return S_OK;
}

void TextService::HandleKey(ITfContext* context, unsigned vk, std::uint32_t key,
                            std::uint32_t mods, BOOL* eaten) {
#ifdef JK_PREDICT
    if (HandleList(context, vk, key, mods)) {
        *eaten = TRUE;
        return;
    }
    // A key that commits takes the reading on screen, not the Korean one underneath.
    const bool commits = (mods & kShortcut) || vk == kVkTab || key == JK_KEY_SPACE ||
                         key == JK_KEY_ENTER || key == JK_KEY_ESCAPE || key == JK_KEY_ARROWUP ||
                         key == JK_KEY_ARROWDOWN || key == JK_KEY_ARROWLEFT ||
                         key == JK_KEY_ARROWRIGHT || key == JK_KEY_OTHER;
    if (shown_ && commits) {
        CommitPrediction(chosen_.value_or(0), context);
        if (key == JK_KEY_SPACE && !(mods & ~static_cast<std::uint32_t>(JK_MOD_SHIFT))) {
            UpdateText(context, L" ", {}, false);
            *eaten = TRUE;
            return;
        }
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
    *eaten = wparam < eaten_down_.size() && eaten_down_[wparam];
    return S_OK;
}

STDMETHODIMP TextService::OnKeyUp(ITfContext*, WPARAM wparam, LPARAM, BOOL* eaten) {
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
    UpdateText(context, commit, preedit, converted);
    ShowList();
}

void TextService::ShowList() {
    const size_t hanja = jk_candidate_count(engine_);
    if (hanja > 0) {
        const size_t page = jk_candidate_page_size();
        const size_t selected = jk_candidate_selected(engine_);
        const size_t start = selected / page * page;
        std::vector<CandidateWindow::Row> rows;
        for (size_t i = start; i < hanja && i < start + page; ++i) {
            rows.push_back({std::to_wstring(i - start + 1),
                            Utf16(jk_candidate_ptr(engine_, i), jk_candidate_len(engine_, i)),
                            Utf16(jk_candidate_comment_ptr(engine_, i),
                                  jk_candidate_comment_len(engine_, i))});
        }
        list_.Show(std::move(rows), static_cast<int>(selected - start),
                   std::to_wstring(selected + 1) + L"/" + std::to_wstring(hanja), anchor_,
                   [this, start](size_t row) { PickHanja(start + row); });
        return;
    }
#ifdef JK_PREDICT
    if (Composing() && predictions_.size() > 1) {
        std::vector<CandidateWindow::Row> rows;
        for (const std::wstring& p : predictions_) rows.push_back({{}, p, {}});
        list_.Show(std::move(rows), chosen_ ? static_cast<int>(*chosen_) : -1, {}, anchor_,
                   [this](size_t row) {
                       if (composition_context_) CommitPrediction(row, composition_context_.Get());
                   });
        return;
    }
#endif
    list_.Hide();
}

// A click on a hanja candidate: move the engine's selection there, then pick it.
void TextService::PickHanja(size_t index) {
    if (!composition_context_) return;
    const size_t selected = jk_candidate_selected(engine_);
    const std::uint32_t key = index > selected ? JK_KEY_ARROWDOWN : JK_KEY_ARROWUP;
    for (size_t i = 0; i < (index > selected ? index - selected : selected - index); ++i) {
        jk_key(engine_, key, 0);
    }
    jk_key(engine_, JK_KEY_ENTER, 0);
    ComPtr<ITfContext> context = composition_context_;
    Apply(context.Get());
}

void TextService::Flush(ITfContext* context) {
#ifdef JK_PREDICT
    if (shown_) {
        CommitPrediction(chosen_.value_or(0), context);
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
    // The mode follows the keyboard, not the app: one setting for every app (spec §8.4).
    SetMode(settings::ReadDword(L"Mode", 0) == 1 ? 1 : 0);
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

void TextService::SetMode(unsigned mode) {
    if (mode == mode_) return;
    mode_ = mode;
    jk_set_mode(engine_, mode);
    if (lang_bar_) lang_bar_->SetState(mode_, open_);
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
    add(kMenuEnglish, L"ABC: Keys Pass Through (한/영)", !open_);
    separator();
    add(kMenuModeA, L"Mode A: JIS Korean (JIS Keyboards)", mode_ == 0);
    add(kMenuModeB, L"Mode B: JIS Korean for US ANSI", mode_ == 1);
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
    switch (id) {
        case kMenuEnglish:
            WriteEnglish(open_);
            return;
        case kMenuModeA:
        case kMenuModeB:
            if (composition_context_) Flush(composition_context_.Get());
            settings::WriteDword(L"Mode", id == kMenuModeB ? 1 : 0);
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
    if (!open && changed && composition_context_) Flush(composition_context_.Get());
    open_ = open;
    // The thread's open/close compartment is what the input indicator and apps read.
    ComPtr<ITfCompartment> openclose;
    if (thread_mgr_ &&
        SUCCEEDED(GetCompartment(thread_mgr_.Get(), GUID_COMPARTMENT_KEYBOARD_OPENCLOSE, &openclose)) &&
        (ReadCompartment(openclose.Get()) != 0) != open) {
        WriteCompartment(openclose.Get(), client_id_, open ? 1 : 0);
    }
    if (lang_bar_) lang_bar_->SetState(mode_, open_);
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
            WriteCompartment(openclose.Get(), client_id_, open_ ? 1 : 0);
        }
    }
    return S_OK;
}

// --- focus and profiles

STDMETHODIMP TextService::OnSetFocus(ITfDocumentMgr*, ITfDocumentMgr*) {
    // Focus moved to another document: what was unconfirmed is committed where it was (spec §5.1).
    if (composition_context_) Flush(composition_context_.Get());
    list_.Hide();
    ApplySettings();  // a setting changed in another app applies here from now on
    return S_OK;
}

STDMETHODIMP TextService::OnActivated(REFCLSID clsid, REFGUID profile, BOOL activated) {
    if (!IsEqualGUID(clsid, kClsidTextService)) return S_OK;
    (void)profile;
    if (activated) {
        jk_activate(engine_);  // leaves any F8 escape (spec §6.1)
        Activated();
    } else if (composition_context_) {
        Flush(composition_context_.Get());
    }
    return S_OK;
}

STDMETHODIMP TextService::OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) {
    // The app ended the composition (a click elsewhere, ...): its text stays as it is shown, and
    // the engine starts fresh.
    if (composition_ && composition == composition_.Get()) {
        ComPtr<ITfRange> range;
        if (composition_context_ && SUCCEEDED(composition->GetRange(&range)) &&
            CaretInside(ec, composition_context_.Get(), range.Get())) {
            CaretToEnd(ec, composition_context_.Get(), range.Get());
        }
        composition_.Reset();
        composition_context_.Reset();
        jk_clear(engine_);
#ifdef JK_PREDICT
        predictions_.clear();
        chosen_.reset();
        shown_.reset();
#endif
        list_.Hide();
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

void TextService::UpdateText(ITfContext* context, const std::wstring& commit,
                             const std::wstring& preedit, bool converted) {
    if (!context) return;
    // A composition left in another document (focus moved) ends there as it is.
    if (composition_ && composition_context_ && !SameObject(composition_context_.Get(), context)) {
        ComPtr<ITfComposition> old = composition_;
        RunEditSession(composition_context_.Get(), client_id_,
                       [old](TfEditCookie ec) { return old->EndComposition(ec); });
        composition_.Reset();
        composition_context_.Reset();
    }
    if (!composition_ && commit.empty() && preedit.empty()) return;
    ComPtr<ITfContext> context_ref = context;
    const TfGuidAtom attribute_atom = converted ? converted_attribute_ : input_attribute_;
    // Captures copies: when TSF can't run the session at once, it runs after this returns.
    auto edit = [this, context_ref, commit, preedit, attribute_atom](TfEditCookie ec) -> HRESULT {
        ITfContext* context = context_ref.Get();
        ComPtr<ITfProperty> attribute;
        context->GetProperty(GUID_PROP_ATTRIBUTE, &attribute);
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
            log::Write(L"no interim selection over the composition");
            return CaretToEnd(ec, context, range);
        };
        auto mark = [&](ITfRange* range) {
            if (attribute && attribute_atom != TF_INVALID_GUIDATOM) {
                VARIANT v;
                v.vt = VT_I4;
                v.lVal = static_cast<LONG>(attribute_atom);
                attribute->SetValue(ec, range, &v);
            }
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

        if (composition_) {
            ComPtr<ITfRange> range;
            composition_->GetRange(&range);
            if (commit.empty() && !preedit.empty()) {  // still composing: just update it
                range->SetText(ec, 0, preedit.data(), static_cast<LONG>(preedit.size()));
                mark(range.Get());
                return show(range.Get());
            }
            // The composition ends; its text becomes `commit` (possibly nothing).
            if (attribute) attribute->Clear(ec, range.Get());
            range->SetText(ec, 0, commit.data(), static_cast<LONG>(commit.size()));
            CaretToEnd(ec, context, range.Get());
            composition_->EndComposition(ec);
            composition_.Reset();
            composition_context_.Reset();
        } else if (!commit.empty()) {
            InsertAtSelection(ec, context, commit);
        }
        if (preedit.empty()) return S_OK;

        ComPtr<ITfInsertAtSelection> insert;
        ComPtr<ITfContextComposition> compositions;
        ComPtr<ITfRange> at;
        if (FAILED(context->QueryInterface(IID_PPV_ARGS(&insert))) ||
            FAILED(context->QueryInterface(IID_PPV_ARGS(&compositions))) ||
            FAILED(insert->InsertTextAtSelection(ec, TF_IAS_QUERYONLY, nullptr, 0, &at))) {
            return E_FAIL;
        }
        HRESULT hr = compositions->StartComposition(ec, at.Get(), this, &composition_);
        if (FAILED(hr) || !composition_) {
            // The app refused a composition: nothing can stay unconfirmed there.
            composition_.Reset();
            jk_flush(engine_);
            return InsertAtSelection(ec, context, preedit);
        }
        composition_context_ = context;
        ComPtr<ITfRange> range;
        composition_->GetRange(&range);
        range->SetText(ec, 0, preedit.data(), static_cast<LONG>(preedit.size()));
        mark(range.Get());
        return show(range.Get());
    };
    const HRESULT hr = RunEditSession(context, client_id_, edit);
    if (FAILED(hr)) log::Write(L"edit session failed %#lx", hr);
}

}  // namespace jk
