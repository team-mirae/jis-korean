// User settings: HKCU\Software\JIS Korean, else HKLM\Software\JIS Korean, else the default.
// HKLM is the fallback for apps that don't see the user's key (FKeyIME measured it in packaged
// apps). Values:
//
//   Predictions        DWORD  1  the predictive layer (spec §10)
//   Mode               DWORD  0  the mode by engine id: 0 A (JIS), 1 B (US ANSI), 2 C (UK ISO)
//   DedicatedSymbols   DWORD  1  Mode C's [ ] and ISO extra key type its symbols (spec §3.6)
//   EnterCommits       DWORD  0  Enter commits without a new line (spec §5.1)
//   LearnFromChoices   DWORD  0  learning from the user's choices (spec §10.11)
//   Learned            SZ        what was learned: one line of counts, no text or keys
//   Weight.<Scenario>  SZ        user tuning (spec §10.5): Korean English Implied Mixed TwoSet
//                                KeepFinals Hanja ImpliedL, e.g. "0.8"
//   DiagnosticLog      DWORD  0  log.h
//   Apps\<exe>         key       per app: Predictions DWORD, overriding the default-off list;
//                                Off DWORD 1: JIS Korean passes every key through there (as ABC)
#pragma once

#include <optional>
#include <string>

namespace jk::settings {

unsigned long ReadDword(const wchar_t* name, unsigned long fallback);
void WriteDword(const wchar_t* name, unsigned long value);
std::optional<std::wstring> ReadString(const wchar_t* name);
void WriteString(const wchar_t* name, const std::wstring& value);
void Delete(const wchar_t* name);

// The running program's file name, lowercase ("notepad.exe").
const std::wstring& ExeName();

// Why predictions start off in `exe` (spec §10.11.1: terminals, remote desktop, password
// managers...), or nullptr when they start on.
const wchar_t* DefaultOffReason(const std::wstring& exe);
// Whether predictions are on in `exe`: the user's choice for it, else the default-off list.
bool AppPredicting(const std::wstring& exe);
void SetAppPredicting(const std::wstring& exe, bool on);

// Whether JIS Korean is off in `exe`: every key passes through, as in ABC, there only. For
// programs whose password boxes don't say they are (banking and certificate programs).
bool AppOff(const std::wstring& exe);
void SetAppOff(const std::wstring& exe, bool off);

}  // namespace jk::settings
