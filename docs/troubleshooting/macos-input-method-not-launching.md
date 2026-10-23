# macOS: the input method stops launching after a reinstall (and Keyboard Viewer legends)

Investigated 2026-10-19 on macOS 27.0 (Darwin 27.0.0), repo at commit `f9c8304` plus uncommitted
work (listed at the end). Times below are local (UTC+9); the input method's own log writes UTC.

**Status (2026-10-19 17:15): cause confirmed.** After fixes B and C were applied and the source was
re-added by hand, the input method launched and worked in System Settings, but TextEdit (running
since two days earlier) still typed ABC and never requested the input method's endpoint. After
quitting and reopening TextEdit, 안녕 came out. So apps that were running when the bundle vanished
keep a stale input-source list, and **neither re-registering nor removing and re-adding the source
refreshes them; only relaunching the app (or logging out) does.** Fix C is kept because the re-add
is what brought the input method back for System Settings, but it is not a recovery for other apps.
Fix B is what prevents the state.

**Update (2026-10-23): fixes B and C shipped; the re-add after every update is still required, and
this is why.** Both landed in `d8d9577`. B is in `mac/Makefile`: `install` now swaps the bundle in
with renames (`ditto` to `.JISKorean.app.new`, `mv` the old aside, `mv` the new into place), so the
path is never absent and the "bundle vanished" state above can't arise from an ordinary reinstall.
C is in `mac/tools/register.swift`: after `TISRegisterInputSource` it snapshots whether the source
was enabled/selected, then disables → settles → enables → reselects the fresh ref, and prints the
remove/re-add instruction if the reselect fails.

C does not refresh **already-running** apps on this macOS. Watching `distnoted` through a real
`make install` and a standalone `register.swift` run showed only observer **registrations** of
`AppleEnabledInputSourcesChangedNotification` (apps subscribing), never a **posted** one — the
programmatic `TISDisableInputSource`/`TISEnableInputSource` pair does not emit the broadcast that
makes running processes reload their input-source catalog. So, to answer "why must I remove and
re-add JIS Korean every time it updates":

> An update runs `killall JISKorean` (SIGTERM — `imklaunchagent` logs `IMDiedCallback … exitStatus
> (15)`) and swaps the bundle. B keeps the path present, so the source is no longer dropped. But
> every app that was already running still holds its cached input-source catalog and its binding to
> the process that was just replaced, and it rebuilds that catalog only on the system-wide
> enabled-sources-changed broadcast — which neither re-registration nor C's programmatic toggle
> sends to it. Removing JIS Korean in System Settings → Keyboard → Input Sources and adding it back
> is a user-initiated enable/disable through the full UI path, which **does** broadcast, so every
> running app re-binds to the fresh process. Logging out and back in rebuilds every catalog from the
> on-disk cache and does the same.

So the state is milder than the 2026-10-19 one (the source is never lost, System Settings and
newly-launched apps are fine immediately), but the manual remove/re-add — or a logout, or quitting
and reopening the specific app — is still what a *running* app needs to pick up the new build. Not
reproduced by live typing this session (same foreground-app caveat as below); concluded from the
install's `distnoted`/`imklaunchagent` evidence on top of the 2026-10-19 analysis.

Caveat for anyone repeating the probes below: a select only makes the *frontmost* app activate the
input method, and only once it has a focused text field and gets a key. Selecting from a script
with no keystroke proves nothing either way. Test by selecting JIS Korean in a text field and typing. The one-paragraph version:

> `make uninstall` followed by a full rebuild left `~/Library/Input Methods/JISKorean.app` absent
> for about a minute while the input source was enabled and selected. The text system noticed the
> bundle disappear and dropped the input source; re-registering it afterwards
> (`TISRegisterInputSource`, what `make install` does) restored it for *new* processes only. Every
> app that was already running, and the text-input agents, kept a catalog without it. Selecting
> JIS Korean then changes the system-wide selection (Hammerspoon's switch returns `true`) but no
> running app activates it, so nothing asks `imklaunchagent` to launch the input method, and keys
> fall through to the previous layout (ABC). Recovering is a matter of making every process reload
> its input-source catalog; the Keyboard Viewer and icon work were never actually exercised because
> of this and are separate, open items.

## Symptoms

- Typing with JIS Korean selected produces Latin letters ("Korean doesn't work").
- No `JISKorean` process (`pgrep -f 'MacOS/JISKorean$'` is empty), no crash report, nothing in the
  input method's own log.
- The input menu's "JIS Korean" section is greyed with a dotted placeholder (the menu agent can't
  reach the input method's process).
- `make install` and `make uninstall && make && make install` don't help; neither does switching
  input sources with F18 (Hammerspoon) or Ctrl+Space.
- Keyboard Viewer shows the ABC (JIS) legends instead of JIS Korean's, and the menu-bar icon is the
  old one. These are the symptoms that started the debugging session; see the last two sections.

## How macOS launches an IMK input method (as observed)

Nothing in the public docs says who launches an input method. The unified log does
(`/usr/bin/log show --info --predicate 'process == "imklaunchagent"'`). A healthy launch, from the
log at 16:19:54 (the last one that the text system performed today):

| Time | Process | Message (abridged) |
|---|---|---|
| 16:19:54.872 | imklaunchagent | `[com.apple.inputmethodkit:Server] Refusing connection name for bundle: unrecognized 'InputMethodConnectionName' value` (an error line; see "Ruled out") |
| 16:19:54.874 | imklaunchagent | `getIMKXPCEndpointForBundle:reply:^() - *Received*` — a client app asks for the input method's XPC endpoint |
| 16:19:54.874 | imklaunchagent | `-getIMKXPCEndpointForBundle: NO Endpoint, Bail & Post to request completion queue!` |
| 16:19:54.874 | imklaunchagent | `(LaunchServices) LaunchServices.launchViaCSUA` / `LAUNCH: Asking CSUI to launch 0 items` |
| 16:19:54.921 | launchd | `Successfully spawned JISKorean[8442] because launch job demand` |
| 16:19:54.923 | CoreServicesUIAgent | `LAUNCH: 0x0-0x443a436 me.sjun.inputmethod.JISKorean starting stopped process.` |
| 16:19:55.380 | imklaunchagent | `Received setIMKXPCEndpoint:forBundleIdentifier: from InputMethod` (pid 8442) then `Calling setIMKXPCEndpoint:forBundleIdentifier: in client apps` |

So: **a client app's text-input session activates the selected input source → asks
`imklaunchagent` for an endpoint → `imklaunchagent` launches the bundle through LaunchServices → the
input method's `IMKServer` registers its endpoint → clients connect.** When the input method dies,
`imklaunchagent` logs `IMDiedCallback for <private>, exitStatus (15)` (15 = SIGTERM from
`killall`, which `make install` runs).

`imklaunchagent` (`/System/Library/Frameworks/InputMethodKit.framework/Resources/imklaunchagent`,
launchd job `com.apple.imklaunchagent`) has been running since login on Oct 16; so have
`TextInputMenuAgent` and `TextInputSwitcher`.

## Timeline with evidence

Ten text-system launches happened between 15:51 and 16:19, one after each `make install`
(`launchd … Successfully spawned JISKorean[…]` at 15:51:45, 15:51:51, 15:54:17, 15:55:12, 15:57:10,
15:57:18, 16:00:08, 16:03:48, 16:07:38, 16:19:54). The learned-choices counter in the preferences
(`Learned`, which only grows while the input method runs and the user commits predictions) went
164 → 167 → 169 → 183 over that period.

| Time | Event | Evidence |
|---|---|---|
| 16:19:54 | Last launch by the text system (pid 8442), after the 16:19 `make install`. | log above |
| 16:21:01 | `make uninstall`: `killall JISKorean`, `rm -rf ~/Library/Input Methods/JISKorean.app`. Then `rm -rf build/full build/lite && make` (full rebuild, roughly a minute), then `make install`. | `imklaunchagent: IMDiedCallback … exitStatus (15)` at 16:21:01.987; `loginwindow: appDeath` |
| 16:21:13 | LaunchServices reacts to the removal. | `com.apple.LaunchServices.applicationUnregistered` for the bundle id |
| 16:21:19–21 | The input menu is rebuilt (menu items changed under the menu agent). | `TextInputMenuAgent … didChangeStateImage: … actual item = (null)` |
| 16:22–16:24 | Reinstalled (bundle present again, `TISRegisterInputSource` called by `tools/register.swift`). User selects JIS Korean and types: Latin comes out. | no spawn, no `JISKorean.log` line, `Learned` frozen at 183 from here on |
| 16:26 | Diagnosis: LaunchServices had four registrations for the bundle id (installed copy, `build/full`, and two paths that no longer existed). Cleaned to one. Did not help. | `lsregister -dump` |
| 16:37:33 | Probe: `open -b me.sjun.inputmethod.JISKorean`. Launches at once, writes `launched from …/Library/Input Methods/JISKorean.app`, stopped afterwards. | log: amfid, syspolicyd (Gatekeeper) evaluate and allow; `launchd … spawned JISKorean[15813]` |
| 16:41:24 | Probe: select JIS Korean through Hammerspoon (`hs.keycodes.currentSourceID(...)`, the F18 path). Returns `true`; a fresh process sees it as the current source. Four seconds later: no process, no log line. Restored ABC. | `imklaunchagent` logs nothing at all between 16:38:17 and now; `CursorUIViewService: -[TUIInputModeAccessory inputMode] should not be nil` at 16:41:24.585 |
| ~16:46–16:49 | Same probe again (the `hs` call hung this time); JIS Korean stayed the selected source for minutes. Still no process. Restored ABC. | same |
| 16:50 | A fresh process sees the live enabled list as ABC, Japanese (two entries), **JIS Korean** — enabled and selectable. `~/Library/Preferences/com.apple.HIToolbox.plist` (written 16:32) lists Apple's 2-Set Korean and not JIS Korean, so that plist is not the live state. | `TISCreateInputSourceList(nil, false)`; `defaults read com.apple.HIToolbox` |

## Root cause

**Confirmed from the log:** after 16:21, selecting the input source never produces an endpoint
request in `imklaunchagent`, so the launch chain never starts. LaunchServices, code signing and the
bundle itself are fine (`open -b` launches it). The break is in the client-side step: the app that
has keyboard focus does not activate the newly selected source.

**Why (inferred; consistent with every observation, not proven by a single line):** the Text Input
Sources catalog is cached per process and updated by notifications. When the bundle vanished for
about a minute, the system treated the input source as removed: LaunchServices unregistered it, the
menu agent rebuilt the menu, and running processes dropped it from their catalogs.
`TISRegisterInputSource` afterwards rebuilt the on-disk cache (`$(getconf DARWIN_USER_CACHE_DIR)/com.apple.IntlDataCache.le*`, mtime 16:24) so that *new* processes see a valid, enabled
source — but it does not tell already-running processes to reload. Those processes (every app the
user types in, the menu agent, the switcher, `imklaunchagent`) still have no usable record for
`me.sjun.inputmethod.JISKorean`. A select changes the system-wide selection, but the focused app has
nothing to activate.

Supporting facts:

- Earlier today the user twice removed and re-added JIS Korean in System Settings (about 16:03 and
  16:12). Enabling/disabling fires the "enabled input sources changed" notification, which makes
  every process reload — and the input method launched normally right after both (16:03:48,
  16:19:54). After 16:21 nobody toggled it, and nothing recovered.
- The symptom is identical for F18 (Hammerspoon), Ctrl+Space and my programmatic select: all three
  only change the selection; none of them refreshes the clients.
- `CursorUIViewService` complained that the input mode for the selected source was nil at the
  moment of the select.
- Every previous `make install` also did `rm -rf` + `cp -R`, but back to back (tens of
  milliseconds), and the text system kept launching the input method. The difference on 16:21 was
  the gap: `make uninstall`, then a full rebuild, then `make install`.

Two neighbouring hypotheses can't be separated from this evidence and would be settled by the
verification steps below:

- The stale party might be `imklaunchagent`'s own table rather than (or as well as) the apps'
  catalogs.
- `TextInputSwitcher` might hold the stale mapping for the session.

## Ruled out

| Suspect | Why it's not the cause |
|---|---|
| Code signing / Gatekeeper (`spctl -a` says "rejected"; amfid logs `-423 … adhoc signed`) | `open -b` launched the same bundle: amfid and syspolicyd evaluated and allowed it. Ad-hoc signing has launched fine all day. |
| LaunchServices duplicate registrations (four paths for one bundle id, two nonexistent) | Launches succeeded with the duplicates present (16:19:54). Cleaning them at 16:26 changed nothing. Still worth keeping clean; see fixes. |
| A crash at launch | No report in `~/Library/Logs/DiagnosticReports` (or `/Library/...`, or `Retired/`), and launchd never spawned the process after 16:21. |
| The Keyboard Viewer layout bundled in the app, `TISIconLabels`, the per-app settings code | The 16:19 build contained all of them and launched normally (pid 8442). |
| `overrideKeyboard(withKeyboardNamed:)` stalling the input method (my theory at 16:19) | The call runs in `activateServer`; it can't prevent a launch, and no build containing it was ever activated. Disabled in the working tree for no good reason — see Keyboard Viewer below. |
| `imklaunchagent`'s "Refusing connection name for bundle: unrecognized 'InputMethodConnectionName' value" | Logged immediately before the successful launches at 16:03:48 and 16:19:54. Apple's own values are `KoreanInputMethodServer_1_Connection` and `com.apple.inputmethod.JIMx.KanaTyping.IMK_Connection`; ours (`me.sjun.inputmethod.JISKorean_Connection`) follows the same shape. Treat it as noise unless a launch is missing *after* it. |
| A leftover launchd/Mach endpoint for the input method | `launchctl print gui/$(id -u)` has no entry for it. |
| The input source not being enabled | The live enabled list (fresh process, `includeAllInstalled = false`) contains it, and it is selectable. The HIToolbox plist disagrees, but that plist is not what the menu uses. |
| Hammerspoon holding a stale reference | Its select returned `true` and the selection changed system-wide; the failure is downstream of the selection. |

## Proposed fixes (none applied)

Ranked by effort. A alone should restore typing; B and C stop it recurring.

### A. Recover now: make every process reload the input-source catalog

1. System Settings → Keyboard → Input Sources → remove **JIS Korean** → add it back (Korean →
   JIS Korean). This fires the enabled-sources-changed notification that re-registration doesn't.
2. Select it (F18) in any app and type.
3. Verify: `/usr/bin/log show --last 2m --info --predicate 'process == "imklaunchagent"'` shows
   `NO Endpoint` → `launchViaCSUA`, `launchd` spawns `JISKorean`, and
   `~/Library/Logs/JISKorean.log` (with `DiagnosticLog` on, see below) gets `launched from …` and `activated in <app bundle id>`.
4. In practice (confirmed 17:10): steps 1–3 reach only the app the source was re-added in. Quit and
   reopen each other app, or log out and back in.
5. If that does nothing: log out and back in (every catalog is rebuilt from the on-disk cache).
   If *that* does nothing, the cause is not the one above; `imklaunchagent`'s log after the select
   is the next thing to read.

Hammerspoon's `init.lua` already works around "a previous switch didn't take" by selecting twice;
it could also alert when `hs.keycodes.currentSourceID(KOREAN)` returns `false`, but in this failure
it returns `true`, so that wouldn't have caught it.

### B. Never leave the installed bundle absent (Makefile)

`mac/Makefile`'s `install` target does `rm -rf` then `cp -R` back to back, which the system
tolerated ten times today. `uninstall` followed by a rebuild does not. Options:

- Reorder clean rebuilds: `rm -rf build && make && make install` (build first, replace last); don't
  run `make uninstall` as part of a reinstall. Cheapest, and only a habit.
- Make `install` atomic: copy to `~/Library/Input Methods/.JISKorean.app.new`, `mv` the old bundle
  aside, `mv` the new one into place, remove the old one. The path is then absent for microseconds
  rather than milliseconds.
- Have `uninstall` disable the input source first (`TISDisableInputSource`), so the removal is an
  orderly disable rather than a vanished bundle. (Not needed if `uninstall` is never used before an
  `install`.)

### C. Make `tools/register.swift` self-healing after a re-registration

After `TISRegisterInputSource`, if the source was enabled before the bundle was replaced (snapshot it
first, or read `AppleInputSourceHistory`), call `TISDisableInputSource` then `TISEnableInputSource`
on the fresh `TISInputSourceRef`, and `TISSelectInputSource` if it was selected. That is the
programmatic form of fix A and should refresh running processes.

Untested points: whether `TISEnableInputSource` succeeds for a third-party input method on
macOS 27 without user interaction (the header only says `paramErr` if it "cannot be enabled"), and
whether enabling from the installer pollutes a first-time install (only do it when it was enabled
before). Verify exactly as in A.

### D. If A works but C can't be made to

Document "remove and re-add the input source after reinstalling" in `mac/README.md` and have
`register.swift` print that instruction when it detects it re-registered a source that the live
enabled list doesn't contain (compare `TISCreateInputSourceList(nil, false)` before and after).

### E. Only if the select *does* reach `imklaunchagent` and still nothing launches

`killall imklaunchagent` (launchd respawns it). Untested; not indicated by today's log, where no
request reached it at all.

## Related, still open: Keyboard Viewer shows ABC legends

What was tried and what the SDK says:

- The input method called `TISSetInputMethodKeyboardLayoutOverride` from its own process on every
  activation. It returned `noErr` (logged ten times between 16:07 and 16:08) and had no visible
  effect. The header (`TextInputSources.h`) says the override "will be used for the final stage of
  key translation in the Window Server" and "is lost when the input method that set it is
  deactivated"; Keyboard Viewer followed the client's layout regardless.
- The documented IMK path is `-[IMKTextInput overrideKeyboardWithKeyboardNamed:]`
  (`IMKInputSession.h`): *"Pass a keyboard unique name. The client will try to locate a keyboard
  layout with that name in the input method's bundle. If a layout is found it is passed to the client
  who then tells the text service manager to use that layout for keyboard events. Input methods
  should call the method each time they are activated."* Apple's Korean input method does exactly
  this (its binary carries `overrideKeyboardWithKeyboardNamed:` and the name `2SetHangul`).
- This is implemented in `mac/Sources/KeyboardViewerLayout.swift` (name `JISKorean`, the `name`
  attribute of the generated `JISKorean.keylayout`), called from `activateServer` in
  `InputController.swift`, with the layout generated from the engine's legends by
  `mac/tools/make-keylayout` and bundled in `Contents/Resources` next to
  `JISKoreanKeyboardLayouts-Info.plist` (Apple's own arrangement). **It has never run:** the call is
  currently commented out to log only (disabled at 16:19 on the mistaken "stall" theory), and no build
  containing it was activated by a client because of the launch problem above.
- `TextInputSources.h` on `TISRegisterInputSource`: *"Input method bundles can include private
  non-selectable keyboard layouts for use with TISSetInputMethodKeyboardLayoutOverride. These are
  registered automatically when the input method is registered."* Observed: right after
  registration the layout is listed (`me.sjun.inputmethod.JISKorean.keylayout.JISKorean`,
  `TISTypeKeyboardLayout`, not selectable) and on a later rescan it is gone. Three times today. This
  matters for the TIS-ref override (which needs a listed layout) and shouldn't for the IMK call,
  which looks the layout up by name inside the bundle.

Proposals, in order:

1. Re-enable the IMK call (one line in `KeyboardViewerLayout.swift`) once the input method launches
   again; check Keyboard Viewer and `JISKorean.log` ("layout JISKorean named to the client").
2. If the client can't find the layout by name, try the id TIS assigns when the layout is listed
   (`me.sjun.inputmethod.JISKorean.keylayout.JISKorean`), or install the layout as a hidden bundle
   in `~/Library/Keyboard Layouts` (`TSMInvisibleKey`, as was tried at 16:06) and pass its id — the
   header's second form, "a DNS type name".
3. If neither works on macOS 27: draw our own keyboard viewer (a panel from the engine's legends, like
   the web playground's), which needs none of this.
4. Or accept ABC legends in Keyboard Viewer and say so in the README.

Whatever the path, keep the generated layout's Cmd, Ctrl and Option layers identical to ABC (JIS)
as the generator does, so shortcuts and Option characters don't change for apps that read the
client's layout.

## Related, still open: the menu-bar icon

Apple's Korean input modes declare a 16×16 `tsInputModeMenuIconFileKey` image and
`TISIconLabels = { Primary = "한" }`; the system draws the standard badge from the label. Ours has
`tsInputMethodIconFileKey = icon.tiff` (16×16, a bare 한 glyph, `TISIconIsTemplate`) and, in the
working tree, the same `TISIconLabels`. In the System Settings "add input source" list ours already
shows the same badge as Apple's (screenshot at 16:12). The menu-bar icon is cached by the menu agent
from registration/enable time, so the plist change can't show until the source is removed and
re-added — which fix A does anyway. If it still differs afterwards, try dropping
`tsInputMethodIconFileKey`/`TISIconIsTemplate` so only the label badge is used (unverified).

## Diagnostic recipes

Gotchas first; three of them cost real time today:

- **`log` is a zsh builtin.** `log show …` in zsh runs the builtin (`log: too many arguments`,
  silently empty in a pipeline). Always write `/usr/bin/log`.
- **`pgrep -f JISKorean.app` matches your own shell's command line** when the command text contains
  that string. Use `pgrep -f 'MacOS/JISKorean$'`.
- **`defaults read com.apple.HIToolbox AppleEnabledInputSources` is not the live state** for
  third-party input methods (it lacked JIS Korean while the menu and TIS had it). Ask TIS from a
  fresh process (below) or look at the input menu.
- Most text-input log messages are `<private>`; filter by process and look at the *shape* of the
  sequence, not the names.

```sh
# Did the text system try to launch it? (the chain in "How macOS launches…")
/usr/bin/log show --last 30m --info --predicate 'process == "imklaunchagent"' --style compact

# Every real launch and death of the input method
/usr/bin/log show --last 2h --info --style compact --predicate \
  '(eventMessage CONTAINS "LAUNCH:" OR eventMessage CONTAINS "Successfully spawned" OR eventMessage CONTAINS "appDeath") AND eventMessage CONTAINS[c] "JISKorean"'

# Can LaunchServices launch the bundle at all? (starts it; stop it afterwards)
open -b me.sjun.inputmethod.JISKorean; sleep 3; cat ~/Library/Logs/JISKorean.log; pkill -f 'MacOS/JISKorean$'

# LaunchServices registrations for the bundle id (should be exactly the installed path)
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump \
  | grep -B3 -A3 'me.sjun.inputmethod.JISKorean' | grep -E '^path:'
```

Live input-source state from a fresh process (`swift state.swift`):

```swift
import Carbon
func id(_ s: TISInputSource) -> String {
    "\(Unmanaged<AnyObject>.fromOpaque(TISGetInputSourceProperty(s, kTISPropertyInputSourceID)!).takeUnretainedValue())"
}
print("current:", id(TISCopyCurrentKeyboardInputSource().takeRetainedValue()))
// false = only the sources that are enabled now (what the input menu shows)
for s in TISCreateInputSourceList(nil, false).takeRetainedValue() as! [TISInputSource] { print("enabled:", id(s)) }
```

The input method's own log, `~/Library/Logs/JISKorean.log`, is the reliable signal: one line per
launch (`launched from <bundle path>`) and per activation (`activated in <client bundle id>`), plus
the Keyboard Viewer step. It's written by `DiagnosticLog.note` and is off by default; turn it on
before diagnosing (it takes effect immediately, no restart):

```sh
defaults write me.sjun.inputmethod.JISKorean DiagnosticLog -bool true    # off again: -bool false
```

## State when this was written

Working tree (uncommitted, on top of `f9c8304`), all built and installed on the author's Mac:

- Engine: `Equal` key types `(` / `)` and `IntlYen` Shift types `^`; `examples/legends.rs`;
  `jk_set_recording` (per-app learning off); tests updated.
- `mac/`: `Sources/KeyboardViewerLayout.swift` (IMK call **disabled**, logs only),
  `Sources/DisabledApps.swift`, `DisabledAppsWindow.swift`, `DefaultOffApps.swift`, per-app menu
  items in `InputController.swift`, `tools/make-keylayout/`, `Resources/JISKoreanKeyboardLayouts-Info.plist`,
  `Info.plist` with `TISIconLabels`, `Makefile` (layout generation and bundling, `lsregister`
  cleanup of build copies, `build/.metadata_never_index`), `tools/register.swift` (waits for the
  bundled layout to be listed, up to 20 s).
- `web/src/stats.json` regenerated for the `(` `)` change.

System-side actions taken during diagnosis on the author's Mac (so nobody hunts for them):

- 16:26: `lsregister -u` on three build copies of the app, `lsregister -f` on the installed copy.
- `~/Library/Keyboard Layouts/JISKorean.bundle` (installed by a 16:06 experiment) removed again by
  later installs.
- Probes: `open -b` at 16:37 (process stopped afterwards); two programmatic selections of
  JIS Korean (16:41, ~16:46), both restored to ABC; no input method process left running.
- `~/Library/Logs/JISKorean.log` created by diagnostic builds (deleted and recreated several times).
- No preference was changed: `Learned`, `TrainingWords.*`, `Weight.*`, `LearnFromChoices` and the
  training file are byte-identical to before the Keyboard Viewer work began.

## Icon: where it ended up (2026-10-19 17:46)

Tried, in order, each with a reinstall and the source re-added:

1. Outlined template 한 as `tsInputMethodIconFileKey` → outline badge.
2. Filled badge with the glyph cut out (pixel-close to Apple's `2SetKorean.tiff`) → still drawn differently.
3. Only `TISIconLabels` (no icon file) → closer, not identical.
4. One input mode in `ComponentInputModeDict` (`me.sjun.inputmethod.JISKorean.2Set`) with mode-level
   `TISIconLabels` and icon keys, as Apple's Korean does; top-level label removed.
5. Icon keys naming `icon.png` while only `icon.tiff` ships (Apple's own mismatch).

Result: close enough and accepted, but the System Settings badge is still a little lighter than
Apple's. Unexplored: whether System Settings draws Apple-signed input methods differently.

Side effect of step 4: the selected source ID is now `me.sjun.inputmethod.JISKorean.2Set`. Anything
comparing the current ID to `me.sjun.inputmethod.JISKorean` (e.g. a Hammerspoon hotkey) must use the
new ID or a prefix match.
