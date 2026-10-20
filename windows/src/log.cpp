#include "log.h"

#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <iterator>

#include "settings.h"

namespace jk::log {
namespace {

const wchar_t* ExeName() {
    static wchar_t name[MAX_PATH] = L"";
    if (!name[0]) {
        wchar_t path[MAX_PATH];
        const DWORD n = GetModuleFileNameW(nullptr, path, MAX_PATH);
        const wchar_t* base = path;
        for (DWORD i = 0; i < n; ++i) {
            if (path[i] == L'\\') base = path + i + 1;
        }
        wcsncpy_s(name, base, _TRUNCATE);
    }
    return name;
}

}  // namespace

void Write(const wchar_t* format, ...) {
    if (!settings::ReadDword(L"DiagnosticLog", 0)) return;
    wchar_t path[MAX_PATH];
    if (!ExpandEnvironmentStringsW(L"%ProgramData%\\JIS Korean\\jiskorean.log", path, MAX_PATH)) {
        return;
    }
    SYSTEMTIME t;
    GetLocalTime(&t);
    wchar_t line[1024];
    int n = swprintf_s(line, L"%02u:%02u:%02u.%03u %lu %s ", t.wHour, t.wMinute, t.wSecond,
                       t.wMilliseconds, GetCurrentProcessId(), ExeName());
    if (n < 0) return;
    va_list args;
    va_start(args, format);
    const int m = vswprintf_s(line + n, std::size(line) - n - 2, format, args);
    va_end(args);
    if (m < 0) return;
    n += m;
    line[n++] = L'\n';
    char utf8[3072];
    const int bytes = WideCharToMultiByte(CP_UTF8, 0, line, n, utf8, sizeof(utf8), nullptr, nullptr);
    if (bytes <= 0) return;
    // FILE_APPEND_DATA: every write lands at the end, so processes don't overwrite each other.
    HANDLE file = CreateFileW(path, FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr,
                              OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return;
    DWORD written;
    WriteFile(file, utf8, static_cast<DWORD>(bytes), &written, nullptr);
    CloseHandle(file);
}

}  // namespace jk::log
