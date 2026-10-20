// COM server and TSF registration (regsvr32, as admin; tools/install.ps1).
#pragma once

#include <windows.h>

namespace jk {

HRESULT RegisterServer(HINSTANCE module);
HRESULT UnregisterServer();
HRESULT RegisterProfiles(HINSTANCE module);
HRESULT UnregisterProfiles();
HRESULT RegisterCategories();
HRESULT UnregisterCategories();

}  // namespace jk
