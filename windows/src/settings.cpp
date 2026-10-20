#include "settings.h"

#include <windows.h>

#include <cwctype>
#include <vector>

namespace jk::settings {
namespace {

constexpr wchar_t kKey[] = L"Software\\JIS Korean";

std::wstring AppKey(const std::wstring& exe) { return std::wstring(kKey) + L"\\Apps\\" + exe; }

// Spec §10.11.1, by program file name: predictions start off where typing is commands, paths,
// passwords or a game's chat.
struct DefaultOff {
    const wchar_t* exe;
    const wchar_t* reason;
};
constexpr wchar_t kTerminal[] = L"a terminal";
constexpr wchar_t kRemote[] = L"file transfer, SSH or remote desktop";
constexpr wchar_t kVm[] = L"containers and virtual machines";
constexpr wchar_t kPasswords[] = L"a password manager";
constexpr wchar_t kLauncher[] = L"File Explorer or a launcher";
constexpr wchar_t kMinecraft[] = L"Minecraft: the JIS Korean mod is the dedicated integration";
constexpr DefaultOff kDefaultOff[] = {
    {L"windowsterminal.exe", kTerminal}, {L"cmd.exe", kTerminal}, {L"conhost.exe", kTerminal},
    {L"openconsole.exe", kTerminal}, {L"powershell.exe", kTerminal}, {L"pwsh.exe", kTerminal},
    {L"powershell_ise.exe", kTerminal}, {L"wsl.exe", kTerminal}, {L"mintty.exe", kTerminal},
    {L"alacritty.exe", kTerminal}, {L"wezterm-gui.exe", kTerminal}, {L"tabby.exe", kTerminal},
    {L"hyper.exe", kTerminal}, {L"putty.exe", kRemote}, {L"kitty.exe", kRemote},
    {L"mobaxterm.exe", kRemote}, {L"termius.exe", kRemote}, {L"winscp.exe", kRemote},
    {L"filezilla.exe", kRemote}, {L"cyberduck.exe", kRemote}, {L"mstsc.exe", kRemote},
    {L"msrdc.exe", kRemote}, {L"windowsapp.exe", kRemote}, {L"parsecd.exe", kRemote},
    {L"anydesk.exe", kRemote}, {L"teamviewer.exe", kRemote}, {L"vncviewer.exe", kRemote},
    {L"vmconnect.exe", kVm}, {L"vmware.exe", kVm}, {L"vmplayer.exe", kVm},
    {L"virtualboxvm.exe", kVm}, {L"docker desktop.exe", kVm},
    {L"keepass.exe", kPasswords}, {L"keepassxc.exe", kPasswords}, {L"1password.exe", kPasswords},
    {L"bitwarden.exe", kPasswords}, {L"explorer.exe", kLauncher}, {L"searchhost.exe", kLauncher},
    {L"powertoys.powerlauncher.exe", kLauncher}, {L"powertoys.cmdpal.ui.exe", kLauncher},
    {L"minecraft.windows.exe", kMinecraft},
};

// Programs JIS Korean is off in by default: the local agents of Korean certificate programs, whose
// password boxes don't mark themselves as password fields. Nobody types ordinary text in them, so
// being off there costs nothing. Measured: running on the owner's PC (2026-10-21) with these
// packages installed; which process shows the password window is inferred, not measured. Third-
// party removal tools' lists (Hoax Eliminator) are "all rights reserved" and aren't used.
constexpr const wchar_t* kOffByDefault[] = {
    L"unicrslocalserver.exe",  // CrossCert UniSign (UniCRS V3)
    L"unicrsnoticenter.exe",   // CrossCert UniSign notices
    L"ccdaemon.exe",           // CrossCert UniSign
    L"magicline4nx.exe",       // DreamSecurity MagicLine4NX
    L"skcertservice.exe",      // SignKorea certificate toolkit
};

}  // namespace

unsigned long ReadDword(const wchar_t* name, unsigned long fallback) {
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD value = 0;
        DWORD size = sizeof(value);
        if (RegGetValueW(root, kKey, name, RRF_RT_REG_DWORD, nullptr, &value, &size) ==
            ERROR_SUCCESS) {
            return value;
        }
    }
    return fallback;
}

void WriteDword(const wchar_t* name, unsigned long value) {
    const DWORD v = value;
    RegSetKeyValueW(HKEY_CURRENT_USER, kKey, name, REG_DWORD, &v, sizeof(v));
}

std::optional<std::wstring> ReadString(const wchar_t* name) {
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD size = 0;
        if (RegGetValueW(root, kKey, name, RRF_RT_REG_SZ, nullptr, nullptr, &size) !=
            ERROR_SUCCESS) {
            continue;
        }
        std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 1);
        size = static_cast<DWORD>(buffer.size() * sizeof(wchar_t));
        if (RegGetValueW(root, kKey, name, RRF_RT_REG_SZ, nullptr, buffer.data(), &size) ==
            ERROR_SUCCESS) {
            return std::wstring(buffer.data());
        }
    }
    return std::nullopt;
}

void WriteString(const wchar_t* name, const std::wstring& value) {
    RegSetKeyValueW(HKEY_CURRENT_USER, kKey, name, REG_SZ, value.c_str(),
                    static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
}

void Delete(const wchar_t* name) { RegDeleteKeyValueW(HKEY_CURRENT_USER, kKey, name); }

const std::wstring& ExeName() {
    static const std::wstring name = [] {
        wchar_t path[MAX_PATH];
        const DWORD n = GetModuleFileNameW(nullptr, path, MAX_PATH);
        std::wstring s(path, n);
        const size_t slash = s.find_last_of(L'\\');
        if (slash != std::wstring::npos) s.erase(0, slash + 1);
        for (wchar_t& c : s) c = static_cast<wchar_t>(std::towlower(c));
        return s;
    }();
    return name;
}

const wchar_t* DefaultOffReason(const std::wstring& exe) {
    for (const DefaultOff& d : kDefaultOff) {
        if (exe == d.exe) return d.reason;
    }
    // Spec §10.11.1: any program named for a terminal, SSH or FTP.
    for (const wchar_t* part : {L"terminal", L"ssh", L"sftp", L"ftp"}) {
        if (exe.find(part) != std::wstring::npos) return kRemote;
    }
    return nullptr;
}

bool AppPredicting(const std::wstring& exe) {
    const std::wstring key = AppKey(exe);
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD value = 0;
        DWORD size = sizeof(value);
        if (RegGetValueW(root, key.c_str(), L"Predictions", RRF_RT_REG_DWORD, nullptr, &value,
                         &size) == ERROR_SUCCESS) {
            return value != 0;
        }
    }
    return DefaultOffReason(exe) == nullptr;
}

bool AppOff(const std::wstring& exe) {
    const std::wstring key = AppKey(exe);
    for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
        DWORD value = 0;
        DWORD size = sizeof(value);
        if (RegGetValueW(root, key.c_str(), L"Off", RRF_RT_REG_DWORD, nullptr, &value, &size) ==
            ERROR_SUCCESS) {
            return value != 0;
        }
    }
    for (const wchar_t* name : kOffByDefault) {
        if (name && exe == name) return true;
    }
    return false;
}

void SetAppOff(const std::wstring& exe, bool off) {
    const DWORD v = off ? 1 : 0;
    RegSetKeyValueW(HKEY_CURRENT_USER, AppKey(exe).c_str(), L"Off", REG_DWORD, &v, sizeof(v));
}

void SetAppPredicting(const std::wstring& exe, bool on) {
    const std::wstring key = AppKey(exe);
    if (on == (DefaultOffReason(exe) == nullptr)) {
        // Back to the default: nothing stays in the settings.
        RegDeleteKeyValueW(HKEY_CURRENT_USER, key.c_str(), L"Predictions");
        RegDeleteKeyW(HKEY_CURRENT_USER, key.c_str());
        return;
    }
    const DWORD v = on ? 1 : 0;
    RegSetKeyValueW(HKEY_CURRENT_USER, key.c_str(), L"Predictions", REG_DWORD, &v, sizeof(v));
}

}  // namespace jk::settings
