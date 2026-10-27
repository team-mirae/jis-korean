// The training contract (docs/training-contract.md, spec §10.12) on Windows, as on macOS
// (mac/Sources/Training.swift): %APPDATA%\JIS Korean\training.json is read when JIS Korean
// activates and the file has changed since. Its weights become the Weight.<Scenario> settings
// (set by hand), its words are kept in the settings and added to the shared predictor, and its
// examples are learned from (turning LearnFromChoices on, since only then do they count).
#pragma once

#include "jis_korean.h"

namespace jk::training {

#ifdef JK_PREDICT
// Sets the predictor on `engine`: the shared one, plus the user's training words if any. Set
// under the lock that guards rebuilding it, so it can't be freed before the engine holds it.
// False: no predictor (the model is missing).
bool SetPredictor(JkEngine* engine);
// Frees the personal predictor (DLL unload, when no engine is left).
void Release();
// Whether the training file changed (or went away) since it was last applied: cheap.
bool Changed();
// Applies the training file if it changed since it was last applied (or forgets its words if
// it's gone). Examples are learned into `engine`; the caller saves what was learned.
void ApplyIfChanged(JkEngine* engine);
// Opens the folder the training file goes in (creating it) in File Explorer.
void OpenFolder();
#endif

}  // namespace jk::training
