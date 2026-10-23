# Interactive installation prompts

Prompts for an AI assistant that walk someone through installing and setting up JIS Korean. The
assistant asks three questions (platform, keyboards, remap tools), then follows the prompt for that
platform.

## Start

In a coding assistant that can read this repository and run commands (Claude Code, for example),
open the repository and say:

```
Read docs/interactive-prompts/start.md and follow it.
```

Without the repository, start a coding assistant in an empty folder and paste [start.md](start.md).
It fetches the repository into that folder first. In a folder that isn't empty, it asks first and
recommends an empty one.

In a chat assistant without access to your computer, paste [start.md](start.md) and then
[rules.md](rules.md). It can't scan anything, so it shows you how to check things yourself.

## Files

| File | What |
|---|---|
| [start.md](start.md) | The three questions and where each answer leads. Read first. |
| [rules.md](rules.md) | Rules for every prompt: ask before scanning, ask before installing, one tool at a time. |
| [macos.md](macos.md) | macOS: the input method, its modes and switch keys, remapping |
| [windows.md](windows.md) | Windows: the TSF input method, 한/영, remapping |
| [minecraft.md](minecraft.md) | Minecraft: the Fabric mod |
| [other.md](other.md) | The web playground, and platforms out of scope |

## Writing a prompt

- Write it to the assistant ("Ask…", "Don't…"), with the questions for the user as quotes.
- Link to the spec and platform READMEs for facts. Don't copy them in: they change, and the copy
  wouldn't.
- Every prompt follows [rules.md](rules.md). Don't add a step that scans or installs without its
  question.
