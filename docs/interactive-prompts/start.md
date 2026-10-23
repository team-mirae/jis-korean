# Set up JIS Korean: start here

You're helping someone install and set up **JIS Korean**, a Korean 2-set (두벌식) input method for
JIS keyboards that also has modes for US ANSI and UK ISO keyboards. First make sure you have the
repository (section 0). Then read [rules.md](rules.md) and follow it throughout. Then ask the three
questions below, one at a time, and follow the prompt the answers lead to.

## 0. Is the repository here?

This file may have been copied and pasted on its own. The other prompts, and the code to build,
are in the repository [team-mirae/jis-korean](https://github.com/team-mirae/jis-korean).

**If you can read files and run commands:** see whether the working directory is the repository: it
has `docs/interactive-prompts/start.md` and `engine/`. The user started this prompt here, so this
one look and the clone below need no question. Everything after them follows rules.md.

- **It's the repository:** go on to rules.md.
- **It's empty** (nothing in it but OS files such as `.DS_Store`, `Thumbs.db` or `desktop.ini`) **and
  it isn't a git repository:** fetch the latest `main` into it, without asking. Pasting this prompt
  already asks for that:

  ```sh
  git init -b main
  git remote add origin https://github.com/team-mirae/jis-korean.git
  git pull origin main
  ```

  If git itself is missing, installing it is a question of its own (rules.md section 2). On macOS,
  running `git` without the Command Line Tools opens Apple's installer: ask before running it.
  Then read rules.md from the clone and go on.
- **Anything else** (other files, another git repository, a parent folder of many projects): don't
  fetch anything. Tell the user what you found, in a line or two (counts and a few names, not a
  full listing), and ask:

  > This folder isn't empty. Setting up here would add the JIS Korean repository (about 20 MB)
  > among these files, and building it writes a few hundred MB more. Your files aren't touched,
  > but tools you use here may pick the new files up (a git repository here, a search index, a sync
  > folder).
  >
  > 1. Recommended: switch to an empty folder and run the prompt again there
  > 2. Continue here anyway, in a new subfolder `jis-korean`

  For 1, say how: make an empty folder, start the assistant there, and paste the prompt again. Then
  stop. For 2, clone into the subfolder (`git clone https://github.com/team-mirae/jis-korean.git`),
  work only inside it, and go on to rules.md.

**If you can't run commands** (a chat assistant): say the setup runs best in a coding assistant
with this repository open. If you can't read rules.md, ask the user to paste it
(<https://github.com/team-mirae/jis-korean/blob/main/docs/interactive-prompts/rules.md>) before
going on. Then go on, and give the user every command to run themselves.

## Opening

Open with one or two sentences on what's going to happen: three questions, then setup steps, and
nothing gets scanned, installed or changed without asking first.

## 1. Platform

> Which platform do you want to type Korean on?
>
> 1. macOS
> 2. Windows
> 3. Minecraft (Java Edition, with Fabric)
> 4. Something else, or just try it first

More than one is fine. Set up one at a time, starting with the one they choose first.

## 2. Keyboards

> Which keyboards will you type on? Any of these works:
>
> 1. The name or model (e.g. "HHKB Studio JIS", "MacBook Air built-in", "MONGDOL 87")
> 2. The layout (JIS, US ANSI, UK ISO, another country's)
> 3. The vendor and product ID, if you know them
> 4. How each one connects (built-in, USB, Bluetooth, through a receiver)
> 5. Or I can look at the keyboards connected to this computer

For 5, ask permission as [rules.md](rules.md) section 1 says, and use the platform prompt's
commands. For each keyboard, find out:
- **its layout:** JIS (¥ left of Backspace, ろ right of /, 英数 and かな or 無変換 and 変換 beside the
  space bar), US ANSI, UK ISO, or another;
- **whether its keys can be remapped in its own firmware** (QMK, VIA, Vial, ZMK, or the maker's own
  app such as HHKB's or Realforce's keymap tool);
- **how it connects.** Some setups behave differently over Bluetooth
  ([#18](https://github.com/team-mirae/jis-korean/issues/18)).

A keyboard with an unknown layout: ask what the key left of Backspace says, and what's right of the
space bar.

## 3. Remap tools

Ask this only on macOS and Windows. Minecraft reads keys itself.

> JIS Korean works without any remapping. Remapping helps when a keyboard lacks a key you want, such
> as a key to switch to Korean, or ¥ and ろ on a non-JIS board. Which software remapping is OK
> with you?
>
> a. Only well-known, non-intrusive tools (Karabiner-Elements on macOS, AutoHotkey or PowerToys
>    on Windows)
> b. A custom stack: as customizable as possible, and I'll maintain and debug it myself
> c. No software remapping: help me remap in the keyboard's firmware, or find another way

## Where to go next

| Platform | Prompt |
|---|---|
| macOS | [macos.md](macos.md) |
| Windows | [windows.md](windows.md) |
| Minecraft | [minecraft.md](minecraft.md) |
| Trying it first, anything else | [other.md](other.md) |

Bring the answers along. Each prompt picks the mode from the keyboards and the switch keys from the
remap answer.

**Mode by keyboard** (spec §3.6; the same on every platform):

| Keyboard | Mode |
|---|---|
| JIS | Mode A, the original layout |
| US ANSI | Mode B, US ANSI symbols kept |
| UK ISO, or another ISO country | Mode C |
| ANSI, remappable, and the user wants the JIS keys | Mode A on a board remapped as JIS: on macOS, [the 87-key guide](../mac-ansi-87-key-recommended/README.md) |
| Several keyboards of different layouts | Add a mode for each. The mode follows the keyboard, so switch modes when you switch keyboards. |
