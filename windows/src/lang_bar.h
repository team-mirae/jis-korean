// The button JIS Korean puts in the input indicator (the taskbar, where the Microsoft Korean IME
// shows 가 / A): 한글 while JIS Korean types, ABC while 한/영 passes keys through. A click
// switches between them; right-click opens the settings menu (the macOS input menu's items), a
// popup the IME shows itself: the indicator only reports the click.
#pragma once

#include <ctfutb.h>
#include <msctf.h>

#include <functional>

namespace jk {

class LangBarButton final : public ITfLangBarItemButton, public ITfSource {
public:
    // `init_menu` fills the menu, `on_menu` runs the item picked, `on_click` a left click.
    LangBarButton(std::function<void(ITfMenu*)> init_menu, std::function<void(UINT)> on_menu,
                  std::function<void()> on_click);
    LangBarButton(const LangBarButton&) = delete;
    LangBarButton& operator=(const LangBarButton&) = delete;

    // `mode` (engine id) names the tooltip; `open` false shows ABC, not 한글.
    void SetState(unsigned mode, bool open);
    void Detach();  // drops the callbacks (the text service is going away)

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** object) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;
    // ITfLangBarItem
    STDMETHODIMP GetInfo(TF_LANGBARITEMINFO* info) override;
    STDMETHODIMP GetStatus(DWORD* status) override;
    STDMETHODIMP Show(BOOL show) override;
    STDMETHODIMP GetTooltipString(BSTR* tooltip) override;
    // ITfLangBarItemButton
    STDMETHODIMP OnClick(TfLBIClick click, POINT pt, const RECT* area) override;
    STDMETHODIMP InitMenu(ITfMenu* menu) override;
    STDMETHODIMP OnMenuSelect(UINT id) override;
    STDMETHODIMP GetIcon(HICON* icon) override;
    STDMETHODIMP GetText(BSTR* text) override;
    // ITfSource
    STDMETHODIMP AdviseSink(REFIID riid, IUnknown* sink, DWORD* cookie) override;
    STDMETHODIMP UnadviseSink(DWORD cookie) override;

private:
    ~LangBarButton();
    LONG refs_ = 1;
    unsigned mode_ = 0;  // for the tooltip
    bool open_ = true;
    std::function<void(ITfMenu*)> init_menu_;
    std::function<void(UINT)> on_menu_;
    std::function<void()> on_click_;
    ITfLangBarItemSink* sink_ = nullptr;
};

}  // namespace jk
