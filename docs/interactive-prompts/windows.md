# Set up JIS Korean on Windows

You come here from [start.md](start.md) with the session notes: official builds or their own,
the keyboards, and the remap answer.
[rules.md](rules.md) applies to every step: ask before each scan, and ask before each install or
change, one at a time.

Facts come from [windows/README.md](../../windows/README.md) and spec §3.6 and §8.4. Read them
before you explain anything, and follow them where this prompt is less precise. Before changing how
keys reach apps, read [windows-tsf-notes.md](../troubleshooting/windows-tsf-notes.md).

## 1. Check what's there (with permission)

Each of these is one read-only check (PowerShell). Ask before running it, as rules.md says, or give
it to the user to run.

| To learn | Command |
|---|---|
| Windows version | `[Environment]::OSVersion.Version` or `winver` |
| Keyboards | `Get-PnpDevice -Class Keyboard -PresentOnly \| Select FriendlyName, InstanceId` (the `VID_`/`PID_` part, and `BTHLE`/`BTHENUM` for Bluetooth) |
| Languages and input methods | `Get-WinUserLanguageList` |
| Build tools (building only) | `cmake --version`, `ninja --version`, `rustup target list --installed` |
| Visual Studio Build Tools (building only) | `& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -products * -format text` |
| JIS Korean already installed | `Test-Path "$env:ProgramFiles\JIS Korean"` |
| Installed by the installer (it's in Apps) | `Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\* -ErrorAction SilentlyContinue \| Where-Object DisplayName -like "JIS Korean*" \| Select DisplayName, DisplayVersion` |
| AutoHotkey or PowerToys running | `Get-Process AutoHotkey*, PowerToys* -ErrorAction SilentlyContinue` |

If the user chose to check things themselves: Settings → System → About (version), Device Manager →
Keyboards (keyboards), Settings → Time & language → Language & region (languages and keyboards).

## 2. Install

### An earlier copy

The installer upgrades an older one, or a development install from `tools/install.ps1`, in place,
and keeps the user's settings: no uninstall needed. Say that apps which were running keep the old
DLL until they restart, and that signing out and back in restarts all of them. The release notes
have the current version of this: follow them where they differ.

### Official builds

The newest release whose tag ends in `-windows` (start.md, "Official builds: the releases").

1. **Download** `JISKorean-<version>-setup.exe` into a new, empty folder, and check its SHA-256
   against the release's `digest` (`Get-FileHash`). This is the only check there is: the installer
   isn't code-signed yet. Say so.
2. **Run it.** That's an install, so ask first, and say what to expect:
   - SmartScreen warns about an unsigned installer: **More info → Run anyway**. That's for this file
     only; never turn SmartScreen off.
   - It asks for **administrator rights (UAC)** once.
   - The wizard asks **Full** (with predictions) or **Lite** (without), and, if they haven't picked
     one before, the starting mode: A for JIS, B for US ANSI, C for UK ISO (start.md's mode table).
   - It copies JIS Korean to Program Files, registers it for 64-bit and 32-bit apps, and adds it to
     the user's Korean keyboards. If Korean isn't in their language list, it adds Korean, without
     removing anything. Say this before they run it: it's a change to their language list.
3. Pick **JIS Korean** with Win+Space, and **restart apps** that were open.

The user runs the installer themselves (it needs their UAC answer). A silent install
(`/VERYSILENT /VARIANT=full|lite /MODE=A|B|C`) is for admins deploying it, not for this.

### Build it yourself

The source is in a folder of its own (start.md section 6). Building needs, each asked about
separately (rules.md section 2):
1. **Visual Studio 2022 Build Tools** with C++ for x64 and x86 (Microsoft, several GB);
2. **CMake** 3.28+;
3. **Ninja**;
4. **Rust** through rustup, with the `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` targets.

winget has all four. If the user prefers another source, use that.

Then, in `windows/`:
- `tools/build.ps1` builds x64 and x86 and runs the shell tests. `-Lite` leaves out predictions.
- Either install it as a development copy, with `tools/install.ps1`. It needs **administrator
  rights (UAC)**: say so before running it. It copies to Program Files, registers the x64 and x86
  DLLs, and adds JIS Korean to the Korean language's keyboards. It adds the Korean language if it's
  missing: that's a change, so ask. `tools/install.ps1 -Update` copies a rebuild in without UAC.
- Or build the installer, as the official builds are: `tools/package.ps1` (needs Inno Setup 6,
  another install to ask about), then run `build/dist/JISKorean-<version>-setup.exe` as above.

Afterwards: pick **JIS Korean** with Win+Space, and **restart apps** that were open. They keep the
old DLL, or none, until they restart.

## 3. Pick the mode

Right-click the 한글 / ABC indicator in the taskbar → Mode A (JIS), Mode B (US ANSI) or Mode C (UK
ISO). It's one setting for every app. Use the mode table in start.md. With keyboards of different
layouts, the mode is switched in the same menu when the keyboard changes.

Mention, without pushing it: the windows README recommends **typing in JIS Korean all the time** and
letting predictions read English words, with a training file for the user's own words, rather than
switching to English with 한/영. Point to that section if they type a lot of English.

## 4. 한/영, by keyboard and remap answer

**US ANSI:** Windows' Korean layout sends 한/영 from Right Alt. Nothing to set up.

**JIS and ISO keyboards have no 한/영 key**, and JIS Korean deliberately doesn't pick one (spec
§8.4). Ask which key they want for it: Right Alt, 変換, 無変換, カタカナ/ひらがな, Caps Lock, or another.
Then go by the remap answer. The indicator's click switches too, on every keyboard, so 한/영 can
also wait.

### a. Non-intrusive tools: PowerToys or AutoHotkey

Ask about one tool, not both. Suggest **PowerToys** (Microsoft) first: its Keyboard Manager maps a
key to 한/영 (`VK_HANGUL`) without a script. **AutoHotkey** is the alternative if they already use
it: a one-line script that sends `vk15`, which they run at login.

Either one only sends 한/영. JIS Korean handles `VK_HANGUL` from wherever it comes.

### b. A custom stack

Everything in a, plus whatever they want to maintain:
- per-app AutoHotkey rules;
- their own mode, forked from Mode A, B or C ([docs/modes.md](../modes.md)), built into their own
  install (`tools/install.ps1 -Update` for development builds);
- settings in `HKCU\Software\JIS Korean` (the README's Settings table): predictions per app,
  `Off` per program, the English layout 한/영 types, the diagnostic log (`tools/install.ps1 -Log`).

Say plainly that they own it: Windows and tool updates can break their scripts, and rebuilds need
apps restarted.

### c. No software remapping

- **The keyboard's firmware or the maker's app** (QMK, VIA, Vial, ZMK, Realforce or HHKB keymap
  tools): send **LANG1** (HID 0x90), which Windows reads as 한/영, from the key they chose.
- **Windows' own Korean keyboard type**, which spec §8.4 lists as an option: it changes how Windows
  reads the right Alt and Ctrl keys. It's a system setting, so it's a change (rules.md section 3),
  and it may need a restart.
- **No remapping at all:** click the indicator to switch.

Changing a keyboard's firmware is a change too. Show the keymap and let the user flash it.

## 5. Check it works

Ask the user to type, in Notepad:
- `ㅎ ㅏ ㄴ` → 한, and `ㄱ ㅏ ㄴ ㅏ` → 가나;
- in Mode A, Shift+1 → 1 and 1 → ㅃ;
- F6 after a word commits it (spec §6);
- 한/영 switches to ABC and back.

Password boxes that don't say they're one (banking and certificate programs) compose like text:
point to the README's "Passwords and screen recording" section. If something fails, the diagnostic
log is `tools/install.ps1 -Log` (ask first: it's a change).

## 6. Wrap up

List what was installed, what was changed, and how to undo each:
- installed with the installer: Settings → Apps → Installed apps → JIS Korean → Uninstall, signed
  in as the account that installed it (the uninstaller can only change the language list of the
  account it runs as). Settings (`HKCU\Software\JIS Korean`) are kept, and so is
  `%APPDATA%\JIS Korean` unless its box is ticked;
- a development install: `tools/install.ps1 -Uninstall` (administrator);
- the Korean language, if it was added, is removed in Settings → Time & language;
- PowerToys and AutoHotkey mappings are undone in the tool, and the tools have their own uninstallers;
- the build tools are removed in Settings → Apps or with winget.

Then offer the Minecraft mod, if they play Minecraft Java Edition ([minecraft.md](minecraft.md)).
