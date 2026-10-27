// The diagnostic log, off by default (setting DiagnosticLog = 1). It records activations, focus
// and errors, never typed text: %ProgramData%\JIS Korean\jiskorean.log.
#pragma once

#include <cstdarg>
#include <cstddef>

namespace jk::log {

void Write(const wchar_t* format, ...);

// Formats into `line` (`size` characters, `used` already taken) and ends it with a newline,
// truncating what doesn't fit: an overlong line must never end the process (the _s functions'
// invalid-parameter handler does). Returns the characters in `line`, not null-terminated.
size_t Append(wchar_t* line, size_t size, size_t used, const wchar_t* format, va_list args);

}  // namespace jk::log
