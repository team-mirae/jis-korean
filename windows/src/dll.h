// The DLL's module handle and lock count (dllmain.cpp).
#pragma once

#include <windows.h>

namespace jk {

HINSTANCE DllInstance();
void DllAddRef();
void DllRelease();

}  // namespace jk
