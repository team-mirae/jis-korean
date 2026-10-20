#include "candidate_window.h"

#include <windowsx.h>

#include <algorithm>

#include "dll.h"

namespace jk {
namespace {

constexpr wchar_t kClassName[] = L"JISKoreanCandidateWindow";

int Scale(int value, UINT dpi) { return MulDiv(value, static_cast<int>(dpi), 96); }

}  // namespace

CandidateWindow::~CandidateWindow() {
    if (hwnd_) DestroyWindow(hwnd_);
    if (font_) DeleteObject(font_);
}

void CandidateWindow::UpdateFont() {
    const UINT dpi = hwnd_ ? GetDpiForWindow(hwnd_) : 96;
    if (font_ && dpi == font_dpi_) return;
    if (font_) DeleteObject(font_);
    NONCLIENTMETRICSW metrics = {sizeof(metrics)};
    SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0, dpi);
    LOGFONTW lf = metrics.lfMessageFont;
    lf.lfHeight = -MulDiv(11, static_cast<int>(dpi), 72);  // 11 pt
    wcscpy_s(lf.lfFaceName, L"Malgun Gothic");            // Korean UI font, on every Windows 10+
    font_ = CreateFontIndirectW(&lf);
    font_dpi_ = dpi;
}

void CandidateWindow::Show(std::vector<Row> rows, int selected, std::wstring footer,
                           const RECT& anchor, std::function<void(size_t)> on_pick) {
    if (!hwnd_) {
        WNDCLASSEXW wc = {sizeof(wc)};
        wc.style = CS_DROPSHADOW;
        wc.lpfnWndProc = Proc;
        wc.hInstance = DllInstance();
        wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        wc.lpszClassName = kClassName;
        RegisterClassExW(&wc);  // fails harmlessly when already registered
        hwnd_ = CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, kClassName,
                                L"", WS_POPUP | WS_BORDER, 0, 0, 1, 1, nullptr, nullptr,
                                DllInstance(), nullptr);
        if (!hwnd_) return;
        SetWindowLongPtrW(hwnd_, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(this));
    }
    rows_ = std::move(rows);
    selected_ = selected;
    footer_ = std::move(footer);
    on_pick_ = std::move(on_pick);

    UpdateFont();
    const UINT dpi = font_dpi_;
    HDC dc = GetDC(hwnd_);
    HGDIOBJ old = SelectObject(dc, font_);
    TEXTMETRICW tm;
    GetTextMetricsW(dc, &tm);
    padding_ = Scale(6, dpi);
    row_height_ = tm.tmHeight + Scale(4, dpi);
    SIZE size;
    const bool labels = std::any_of(rows_.begin(), rows_.end(), [](const Row& r) { return !r.label.empty(); });
    GetTextExtentPoint32W(dc, L"9", 1, &size);
    label_width_ = labels ? size.cx + Scale(10, dpi) : 0;
    text_width_ = 0;
    int width = 0;
    for (const Row& r : rows_) {
        GetTextExtentPoint32W(dc, r.text.c_str(), static_cast<int>(r.text.size()), &size);
        text_width_ = std::max(text_width_, static_cast<int>(size.cx));
    }
    for (const Row& r : rows_) {
        int w = text_width_;
        if (!r.comment.empty()) {
            GetTextExtentPoint32W(dc, r.comment.c_str(), static_cast<int>(r.comment.size()), &size);
            w += Scale(12, dpi) + size.cx;
        }
        width = std::max(width, w);
    }
    GetTextExtentPoint32W(dc, footer_.c_str(), static_cast<int>(footer_.size()), &size);
    width = std::max(width, static_cast<int>(size.cx) - label_width_);
    SelectObject(dc, old);
    ReleaseDC(hwnd_, dc);

    width += padding_ * 2 + label_width_ + Scale(2, dpi);
    const int lines = static_cast<int>(rows_.size()) + (footer_.empty() ? 0 : 1);
    const int height = padding_ * 2 + row_height_ * lines + Scale(2, dpi);

    // Below the text, kept on its monitor; above it when it doesn't fit below.
    MONITORINFO mi = {sizeof(mi)};
    GetMonitorInfoW(MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST), &mi);
    const int x = std::clamp(static_cast<int>(anchor.left), static_cast<int>(mi.rcWork.left),
                             std::max(static_cast<int>(mi.rcWork.left),
                                      static_cast<int>(mi.rcWork.right) - width));
    int y = anchor.bottom + Scale(2, dpi);
    if (y + height > mi.rcWork.bottom) y = anchor.top - height - Scale(2, dpi);
    SetWindowPos(hwnd_, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    InvalidateRect(hwnd_, nullptr, TRUE);
}

void CandidateWindow::Hide() {
    if (hwnd_) ShowWindow(hwnd_, SW_HIDE);
    on_pick_ = nullptr;
}

void CandidateWindow::Paint(HDC dc) {
    RECT client;
    GetClientRect(hwnd_, &client);
    FillRect(dc, &client, GetSysColorBrush(COLOR_WINDOW));
    HGDIOBJ old = SelectObject(dc, font_);
    SetBkMode(dc, TRANSPARENT);
    int y = padding_;
    for (size_t i = 0; i < rows_.size(); ++i) {
        const Row& r = rows_[i];
        const bool focused = static_cast<int>(i) == selected_;
        RECT row = {client.left + padding_ / 2, y, client.right - padding_ / 2, y + row_height_};
        if (focused) FillRect(dc, &row, GetSysColorBrush(COLOR_HIGHLIGHT));
        const int flags = DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_NOPREFIX;
        SetTextColor(dc, GetSysColor(focused ? COLOR_HIGHLIGHTTEXT : COLOR_GRAYTEXT));
        RECT label = {client.left + padding_, y, client.left + padding_ + label_width_, y + row_height_};
        DrawTextW(dc, r.label.c_str(), -1, &label, flags);
        SetTextColor(dc, GetSysColor(focused ? COLOR_HIGHLIGHTTEXT : COLOR_WINDOWTEXT));
        RECT text = {label.right, y, client.right - padding_, y + row_height_};
        DrawTextW(dc, r.text.c_str(), -1, &text, flags);
        if (!r.comment.empty()) {
            SetTextColor(dc, GetSysColor(focused ? COLOR_HIGHLIGHTTEXT : COLOR_GRAYTEXT));
            RECT comment = {label.right + text_width_ + MulDiv(12, font_dpi_, 96), y,
                            client.right - padding_, y + row_height_};
            DrawTextW(dc, r.comment.c_str(), -1, &comment, flags);
        }
        y += row_height_;
    }
    if (!footer_.empty()) {
        SetTextColor(dc, GetSysColor(COLOR_GRAYTEXT));
        RECT footer = {client.left + padding_, y, client.right - padding_, y + row_height_};
        DrawTextW(dc, footer_.c_str(), -1, &footer, DT_SINGLELINE | DT_VCENTER | DT_RIGHT);
    }
    SelectObject(dc, old);
}

LRESULT CALLBACK CandidateWindow::Proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    auto* self = reinterpret_cast<CandidateWindow*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    switch (message) {
        case WM_MOUSEACTIVATE:
            return MA_NOACTIVATE;  // clicks never take focus from the app
        case WM_LBUTTONUP:
            if (self && self->row_height_ > 0 && self->on_pick_) {
                const int row = (GET_Y_LPARAM(lparam) - self->padding_) / self->row_height_;
                if (row >= 0 && row < static_cast<int>(self->rows_.size())) {
                    auto pick = self->on_pick_;  // may hide (and reset) the window
                    pick(static_cast<size_t>(row));
                }
            }
            return 0;
        case WM_PAINT: {
            PAINTSTRUCT ps;
            HDC dc = BeginPaint(hwnd, &ps);
            if (self) self->Paint(dc);
            EndPaint(hwnd, &ps);
            return 0;
        }
        case WM_ERASEBKGND:
            return 1;
    }
    return DefWindowProcW(hwnd, message, wparam, lparam);
}

}  // namespace jk
