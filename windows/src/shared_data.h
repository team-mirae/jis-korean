// The hanja dictionary and the predictor, parsed once per process from the DLL's resources and
// shared by every engine (they're immutable). Parsing takes about 100 ms (measured), so Preload
// starts it on a thread-pool thread when JIS Korean activates; the getters wait for it if needed.
#pragma once

#include "jis_korean.h"

namespace jk {

void Preload();
const JkDict* SharedDictionary();
#ifdef JK_PREDICT
const JkPredictor* SharedPredictor();  // nullptr if the model is missing
#endif
// Runs `job` on a thread-pool thread; the DLL stays loaded until it has returned. False: not queued.
bool RunInBackground(void (*job)());
// Frees the shared data: DLL unload only, when no engine is left.
void ReleaseSharedData();

}  // namespace jk
