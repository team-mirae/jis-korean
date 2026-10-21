#!/bin/sh
# Adds JIS English (mode A_QWERTY, spec §3.6.2, §8.2.1) to a built bundle's Info.plist and Keyboard
# Viewer layout list. Run by `make ENGLISH=1`; the checked-in Info.plist has the Korean modes only.
# Usage: add-english-mode.sh <JISKorean.app/Contents>
set -eu
plist="$1/Info.plist"
layouts="$1/Resources/JISKoreanKeyboardLayouts-Info.plist"
id=me.sjun.inputmethod.JISKorean.A_QWERTY
mode=":ComponentInputModeDict:tsInputModeListKey:$id"
pb=/usr/libexec/PlistBuddy

# English, so System Settings lists it under English; Roman script, primary in it, as Japanese
# input methods declare their Romaji modes.
$pb -c "Add $mode dict" \
    -c "Add $mode:TISInputSourceID string $id" \
    -c "Add $mode:TISIntendedLanguage string en" \
    -c "Add $mode:tsInputModeIsVisibleKey bool true" \
    -c "Add $mode:tsInputModeMenuIconFileKey string mode-a-qwerty.png" \
    -c "Add $mode:tsInputModePaletteIconFileKey string mode-a-qwerty.png" \
    -c "Add $mode:tsInputModePrimaryInScriptKey bool true" \
    -c "Add $mode:tsInputModeScriptKey string smRoman" \
    -c "Add :ComponentInputModeDict:tsVisibleInputModeOrderedArrayKey: string $id" \
    -c "Add :tsInputMethodCharacterRepertoireKey: string Latn" \
    "$plist"

$pb -c "Add :KLInfo_JISEnglish dict" \
    -c "Add :KLInfo_JISEnglish:ISOCode string en" \
    -c "Add :KLInfo_JISEnglish:TSMInvisibleKey bool true" \
    "$layouts"
