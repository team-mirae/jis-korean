// The diagnostic log, off by default (setting DiagnosticLog = 1). It records activations, focus
// and errors, never typed text: %ProgramData%\JIS Korean\jiskorean.log.
#pragma once

namespace jk::log {

void Write(const wchar_t* format, ...);

}  // namespace jk::log
