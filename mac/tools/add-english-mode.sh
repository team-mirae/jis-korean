#!/bin/sh
# Adds the English modes (spec §3.6.2, §8.2.1) to a built bundle's Info.plist and Keyboard Viewer
# layout list: JIS English (A_QWERTY), US English (B_QWERTY) and UK English (C_QWERTY). Run by
# `make ENGLISH=1`; the checked-in Info.plist has the Korean modes only.
# Usage: add-english-mode.sh <JISKorean.app/Contents>
set -eu
plist="$1/Info.plist"
layouts="$1/Resources/JISKoreanKeyboardLayouts-Info.plist"
pb=/usr/libexec/PlistBuddy

# name, badge, Keyboard Viewer layout, primary in the Roman script (only one mode can be)
add() {
    id="me.sjun.inputmethod.JISKorean.$1"
    mode=":ComponentInputModeDict:tsInputModeListKey:$id"
    # English, so System Settings lists it under English; Roman script, as Japanese input methods
    # declare their Romaji modes.
    $pb -c "Add $mode dict" \
        -c "Add $mode:TISInputSourceID string $id" \
        -c "Add $mode:TISIntendedLanguage string en" \
        -c "Add $mode:tsInputModeIsVisibleKey bool true" \
        -c "Add $mode:tsInputModeMenuIconFileKey string $2.png" \
        -c "Add $mode:tsInputModePaletteIconFileKey string $2.png" \
        -c "Add $mode:tsInputModePrimaryInScriptKey bool $4" \
        -c "Add $mode:tsInputModeScriptKey string smRoman" \
        -c "Add :ComponentInputModeDict:tsVisibleInputModeOrderedArrayKey: string $id" \
        "$plist"
    $pb -c "Add :KLInfo_$3 dict" \
        -c "Add :KLInfo_$3:ISOCode string en" \
        -c "Add :KLInfo_$3:TSMInvisibleKey bool true" \
        "$layouts"
}

add A_QWERTY mode-a-qwerty JISEnglish true
add B_QWERTY mode-b-qwerty USEnglish false
add C_QWERTY mode-c-qwerty UKEnglish false
$pb -c "Add :tsInputMethodCharacterRepertoireKey: string Latn" "$plist"
