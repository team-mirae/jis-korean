// The JIS Korean text service: one COM object per thread that has TSF, holding one engine. All
// behavior is in the engine (docs/spec.md): this class moves key events in, and text out as a
// TSF composition, and shows the hanja / prediction list. Mirrors mac/Sources/InputController.swift.
//
// 한/영 (VK_HANGUL) switches JIS Korean off and on, as the Microsoft IME does: while off, every
// key goes to the app untouched. The state is one per session (a TSF global compartment), never
// stored: it starts on Korean after a restart.
#pragma once

#include <msctf.h>
#include <windows.h>
#include <wrl/client.h>

#include <bitset>
#include <optional>
#include <string>
#include <vector>

#include "candidate_window.h"
#include "jis_korean.h"
#include "lang_bar.h"

namespace jk {

class TextService final : public ITfTextInputProcessorEx,
                          public ITfKeyEventSink,
                          public ITfThreadMgrEventSink,
                          public ITfActiveLanguageProfileNotifySink,
                          public ITfCompositionSink,
                          public ITfCompartmentEventSink,
                          public ITfDisplayAttributeProvider {
public:
    TextService();
    TextService(const TextService&) = delete;
    TextService& operator=(const TextService&) = delete;

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** object) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // ITfTextInputProcessor(Ex)
    STDMETHODIMP Activate(ITfThreadMgr* thread_mgr, TfClientId client_id) override;
    STDMETHODIMP ActivateEx(ITfThreadMgr* thread_mgr, TfClientId client_id, DWORD flags) override;
    STDMETHODIMP Deactivate() override;

    // ITfKeyEventSink
    STDMETHODIMP OnSetFocus(BOOL foreground) override;
    STDMETHODIMP OnTestKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam,
                               BOOL* eaten) override;
    STDMETHODIMP OnTestKeyUp(ITfContext* context, WPARAM wparam, LPARAM lparam,
                             BOOL* eaten) override;
    STDMETHODIMP OnKeyDown(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnKeyUp(ITfContext* context, WPARAM wparam, LPARAM lparam, BOOL* eaten) override;
    STDMETHODIMP OnPreservedKey(ITfContext* context, REFGUID guid, BOOL* eaten) override;

    // ITfThreadMgrEventSink
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr*) override { return S_OK; }
    STDMETHODIMP OnSetFocus(ITfDocumentMgr* focus, ITfDocumentMgr* previous) override;
    STDMETHODIMP OnPushContext(ITfContext*) override { return S_OK; }
    STDMETHODIMP OnPopContext(ITfContext*) override { return S_OK; }

    // ITfActiveLanguageProfileNotifySink
    STDMETHODIMP OnActivated(REFCLSID clsid, REFGUID profile, BOOL activated) override;

    // ITfCompositionSink
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ec, ITfComposition* composition) override;

    // ITfCompartmentEventSink
    STDMETHODIMP OnChange(REFGUID guid) override;

    // ITfDisplayAttributeProvider
    STDMETHODIMP EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** infos) override;
    STDMETHODIMP GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo** info) override;

private:
    ~TextService();

    // Whether a key-down goes through the engine. Decided the same way in OnTestKeyDown and
    // OnKeyDown; keys the engine then doesn't consume go on to the app after its commit.
    bool Wants(unsigned vk, std::uint32_t key, std::uint32_t mods) const;
    // A password or PIN field (input scope), or one the app turned the keyboard off for.
    bool PrivateField(ITfContext* context);
    bool Private(const wchar_t* why);  // logs why a field is private, returns true
    bool Composing() const;
    void HandleKey(ITfContext* context, unsigned vk, std::uint32_t key, std::uint32_t mods,
                   BOOL* eaten);

    // Inserts the engine's commit text, then shows its preedit (or the reading chosen in its
    // place) as the composition, and updates the list.
    void Apply(ITfContext* context);
    void Refresh(ITfContext* context, const std::wstring& commit = {});
    void ShowList();
    void PickHanja(size_t index);
    // Commits everything unconfirmed (focus loss, IME switch, 한/영).
    void Flush(ITfContext* context);
    void ApplySettings();
    void Activated();  // settings, then a changed training file (spec §10.12)
    void EnsureData();  // the dictionary and predictor, before the first key
    void SetMode(unsigned mode);  // 0 = Mode A (JIS), 1 = Mode B (US ANSI), spec §3.6

#ifdef JK_PREDICT
    // ↓ / Tab and ↑ move through the predictions, Escape leaves the list, Enter commits the
    // chosen row. False: not a list key.
    bool HandleList(ITfContext* context, unsigned vk, std::uint32_t key, std::uint32_t mods);
    void CommitPrediction(size_t index, ITfContext* context);
    void SaveLearned();
#endif

    // 한/영: the session-wide English state, and this thread's open/close compartment (what the
    // input indicator shows).
    bool ReadEnglish();
    void WriteEnglish(bool english);
    void SetOpen(bool open);
    void AdviseCompartments(bool on);
    void InitMenu(ITfMenu* menu);
    void OnMenu(UINT id);

    // Replaces the composition (if any) with `commit`, which becomes final text, then starts or
    // updates a composition showing `preedit`. Either may be empty. `converted`: the preedit is a
    // reading in place of the typed one (thick underline).
    void UpdateText(ITfContext* context, const std::wstring& commit, const std::wstring& preedit,
                    bool converted);

    LONG refs_ = 1;
    JkEngine* engine_;
    Microsoft::WRL::ComPtr<ITfThreadMgr> thread_mgr_;
    TfClientId client_id_ = TF_CLIENTID_NULL;
    DWORD thread_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD profile_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD english_sink_cookie_ = TF_INVALID_COOKIE;
    DWORD openclose_sink_cookie_ = TF_INVALID_COOKIE;
    TfGuidAtom input_attribute_ = TF_INVALID_GUIDATOM;
    TfGuidAtom converted_attribute_ = TF_INVALID_GUIDATOM;
    Microsoft::WRL::ComPtr<ITfComposition> composition_;
    Microsoft::WRL::ComPtr<ITfContext> composition_context_;
    RECT anchor_ = {};  // the composition on screen, for the list
    CandidateWindow list_;
    LangBarButton* lang_bar_ = nullptr;
    std::bitset<256> eaten_down_;  // key-downs eaten, so their key-ups are eaten too
    // A key-down OnTestKeyDown handled and reported as eaten; OnKeyDown follows for it.
    struct Tested {
        WPARAM wparam = 0;
        LPARAM lparam = 0;
        ULONGLONG tick = 0;
        BOOL eaten = FALSE;
        bool pending = false;
    } tested_;
    unsigned mode_ = 0xFF;  // none yet: the first ApplySettings sets it
    bool open_ = true;  // false: 한/영 switched to English
    bool predicting_ = false;
    bool private_ = false;     // the focused field is a password field: keys pass through
    bool app_off_ = false;     // JIS Korean is off in this program (settings::AppOff)
    std::wstring last_scopes_;  // for the log
    bool data_ready_ = false;  // the shared dictionary and predictor are set on the engine
#ifdef JK_PREDICT
    std::vector<std::wstring> predictions_;
    std::optional<size_t> chosen_;     // the row chosen with ↓ / Tab
    std::optional<std::wstring> shown_;  // shown in place of the Korean reading
#endif
};

}  // namespace jk
