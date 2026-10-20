#include "lang_bar.h"

#include <olectl.h>

#include <algorithm>
#include <string>
#include <vector>

#include "dll.h"
#include "guids.h"

namespace jk {
namespace {

constexpr DWORD kSinkCookie = 1;

// The taskbar's theme, so the label reads like the Microsoft IME's: dark text on a light taskbar.
bool LightTaskbar() {
    DWORD value = 0;
    DWORD size = sizeof(value);
    return RegGetValueW(HKEY_CURRENT_USER,
                        L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                        L"SystemUsesLightTheme", RRF_RT_REG_DWORD, nullptr, &value,
                        &size) == ERROR_SUCCESS &&
           value != 0;
}

// `text` drawn as a square icon of `size` pixels, as large as fits.
HICON TextIcon(const wchar_t* text, int size) {
    BITMAPINFO bi = {};
    bi.bmiHeader.biSize = sizeof(bi.bmiHeader);
    bi.bmiHeader.biWidth = size;
    bi.bmiHeader.biHeight = -size;  // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;
    void* bits = nullptr;
    HDC screen = GetDC(nullptr);
    HDC dc = CreateCompatibleDC(screen);
    ReleaseDC(nullptr, screen);
    HBITMAP color = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (!dc || !color || !bits) {
        if (color) DeleteObject(color);
        if (dc) DeleteDC(dc);
        return nullptr;
    }
    HGDIOBJ old_bitmap = SelectObject(dc, color);
    auto* pixels = static_cast<DWORD*>(bits);
    std::fill(pixels, pixels + size * size, 0);

    // White on black with grayscale antialiasing: each pixel's brightness is its coverage.
    const int length = static_cast<int>(wcslen(text));
    HFONT font = nullptr;
    for (int height = size; height > 4; --height) {
        if (font) DeleteObject(font);
        font = CreateFontW(-height, 0, 0, 0, FW_SEMIBOLD, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                           OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY,
                           DEFAULT_PITCH, L"Malgun Gothic");
        SelectObject(dc, font);
        SIZE extent;
        GetTextExtentPoint32W(dc, text, length, &extent);
        if (extent.cx <= size) break;
    }
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, RGB(255, 255, 255));
    RECT rect = {0, 0, size, size};
    DrawTextW(dc, text, length, &rect, DT_SINGLELINE | DT_CENTER | DT_VCENTER | DT_NOPREFIX);
    GdiFlush();

    const BYTE ink = LightTaskbar() ? 0 : 255;
    for (int i = 0; i < size * size; ++i) {
        const DWORD p = pixels[i];
        const BYTE a = static_cast<BYTE>(std::max({p & 0xFF, (p >> 8) & 0xFF, (p >> 16) & 0xFF}));
        const BYTE c = static_cast<BYTE>(ink * a / 255);  // premultiplied
        pixels[i] = (static_cast<DWORD>(a) << 24) | (c << 16) | (c << 8) | c;
    }
    SelectObject(dc, old_bitmap);
    if (font) DeleteObject(font);
    DeleteDC(dc);

    std::vector<BYTE> zeros(static_cast<size_t>((size + 15) / 16 * 2 * size), 0);
    HBITMAP mask = CreateBitmap(size, size, 1, 1, zeros.data());
    ICONINFO info = {TRUE, 0, 0, mask, color};
    HICON icon = CreateIconIndirect(&info);
    DeleteObject(mask);
    DeleteObject(color);
    return icon;
}

}  // namespace

LangBarButton::LangBarButton(std::function<void(ITfMenu*)> init_menu,
                             std::function<void(UINT)> on_menu, std::function<void()> on_click)
    : init_menu_(std::move(init_menu)),
      on_menu_(std::move(on_menu)),
      on_click_(std::move(on_click)) {
    DllAddRef();
}

LangBarButton::~LangBarButton() {
    if (sink_) sink_->Release();
    DllRelease();
}

void LangBarButton::SetState(unsigned mode, bool open) {
    if (mode == mode_ && open == open_) return;
    mode_ = mode;
    open_ = open;
    if (sink_) sink_->OnUpdate(TF_LBI_ICON | TF_LBI_TOOLTIP | TF_LBI_TEXT);
}

void LangBarButton::Detach() {
    init_menu_ = nullptr;
    on_menu_ = nullptr;
    on_click_ = nullptr;
}

STDMETHODIMP LangBarButton::QueryInterface(REFIID riid, void** object) {
    if (!object) return E_INVALIDARG;
    if (riid == IID_IUnknown || riid == IID_ITfLangBarItem || riid == IID_ITfLangBarItemButton) {
        *object = static_cast<ITfLangBarItemButton*>(this);
    } else if (riid == IID_ITfSource) {
        *object = static_cast<ITfSource*>(this);
    } else {
        *object = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) LangBarButton::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) LangBarButton::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (!refs) delete this;
    return refs;
}

STDMETHODIMP LangBarButton::GetInfo(TF_LANGBARITEMINFO* info) {
    if (!info) return E_INVALIDARG;
    info->clsidService = kClsidTextService;
    // GUID_LBI_INPUTMODE: Windows 8 and later show this button in the taskbar's input indicator.
    info->guidItem = kLangBarItemInputMode;
    info->dwStyle = TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY;
    info->ulSort = 0;
    wcscpy_s(info->szDescription, L"JIS Korean");
    return S_OK;
}

STDMETHODIMP LangBarButton::GetStatus(DWORD* status) {
    if (!status) return E_INVALIDARG;
    *status = 0;
    return S_OK;
}

STDMETHODIMP LangBarButton::Show(BOOL) { return E_NOTIMPL; }

STDMETHODIMP LangBarButton::GetTooltipString(BSTR* tooltip) {
    if (!tooltip) return E_INVALIDARG;
    std::wstring text = mode_ == 1 ? L"JIS Korean, Mode B (US ANSI)" : L"JIS Korean, Mode A (JIS)";
    text += open_ ? L": 한글" : L": ABC (한/영)";
    *tooltip = SysAllocString(text.c_str());
    return *tooltip ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP LangBarButton::OnClick(TfLBIClick click, POINT, const RECT*) {
    if (click == TF_LBI_CLK_LEFT && on_click_) on_click_();
    return S_OK;
}

STDMETHODIMP LangBarButton::InitMenu(ITfMenu* menu) {
    if (menu && init_menu_) init_menu_(menu);
    return S_OK;
}

STDMETHODIMP LangBarButton::OnMenuSelect(UINT id) {
    if (on_menu_) on_menu_(id);
    return S_OK;
}

STDMETHODIMP LangBarButton::GetIcon(HICON* icon) {
    if (!icon) return E_INVALIDARG;
    *icon = TextIcon(open_ ? L"한글" : L"ABC", GetSystemMetrics(SM_CXSMICON));
    return *icon ? S_OK : E_FAIL;
}

STDMETHODIMP LangBarButton::GetText(BSTR* text) {
    if (!text) return E_INVALIDARG;
    *text = SysAllocString(open_ ? L"한글" : L"ABC");
    return *text ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP LangBarButton::AdviseSink(REFIID riid, IUnknown* sink, DWORD* cookie) {
    if (!sink || !cookie) return E_INVALIDARG;
    if (riid != IID_ITfLangBarItemSink) return CONNECT_E_CANNOTCONNECT;
    if (sink_) return CONNECT_E_ADVISELIMIT;
    if (FAILED(sink->QueryInterface(IID_PPV_ARGS(&sink_)))) {
        sink_ = nullptr;
        return E_NOINTERFACE;
    }
    *cookie = kSinkCookie;
    return S_OK;
}

STDMETHODIMP LangBarButton::UnadviseSink(DWORD cookie) {
    if (cookie != kSinkCookie || !sink_) return CONNECT_E_NOCONNECTION;
    sink_->Release();
    sink_ = nullptr;
    return S_OK;
}

}  // namespace jk
