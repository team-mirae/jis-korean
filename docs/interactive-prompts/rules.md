# Rules for every interactive prompt

These apply from the first question to the last step, in every prompt in this folder. If a step in
another prompt seems to conflict with them, these win.

## 1. Ask before scanning anything

Ask before you read anything on the user's system: before any command, file read or query, even
read-only ones (listing keyboards, reading a preference, checking whether an app runs). Say what
you'd run and why, then offer exactly these three choices:

> I'd like to check *what*, by running `the command`. It only reads; nothing changes.
>
> 1. Yes, this once
> 2. Yes, and allow read-only checks for the rest of this session
> 3. No: tell me how to check it myself

- **1:** run that one check. Ask again for the next one.
- **2:** run read-only checks without asking again, for this session only. Still ask before
  anything that changes the system (section 2), and say what each check was for as you run it.
- **3:** give the command or the clicks (System Settings → …, Device Manager → …), say what to look
  for in the result, and wait for the user's answer. Don't ask about scanning again unless they
  bring it up.

Read-only means the command can't change anything: no `sudo`, no writes, no installs, no settings.
When unsure, treat it as a change.

One exception, in [start.md](start.md) section 0: when the prompt was pasted without the repository,
one look at the working directory and fetching the repository into it when it's empty need no
question. The user already asked for that by pasting the prompt. Nothing else is exempt.

When you report what you found, leave out serial numbers, Bluetooth and network addresses, user
names and computer names. Keep this in mind for anything that might end up in an issue or a
shared log.

## 2. Ask before installing anything, one tool at a time

Ask before you install any software, separately for each one, and wait for the answer before you
bring up the next. Never ask about two in one question.

- Right: "Can I install Karabiner-Elements?"
- Wrong: "Can I install Karabiner-Elements, Mozc, and another JIS input method I found that would fit
  you well?"

Each question says:
- what the tool is, and what it's needed for here;
- where it comes from (the official site, Homebrew, winget, …);
- whether it needs an administrator password, a restart or a permission (Accessibility, Input
  Monitoring, UAC);
- how to remove it.

A "no" is final for that tool. Take the next-best route, or say plainly what won't work without it.
Don't suggest tools the user didn't ask about, and don't present a tool from a web search as
recommended unless this repository's docs recommend it.

## 3. Ask before changing settings

Changing a setting works the same as installing: one change per question, saying what it changes
and how to undo it. This covers system preferences, keyboard types, registry values, input sources,
login items, and a keyboard's firmware or keymap. A change that needs `sudo` or an administrator
prompt is one the user runs: give them the command, and don't try to work around the password.

## 4. Everything else

- Ask one question at a time, with numbered choices where the answers are known.
- Use what the user already told you. Don't ask again for something they've answered.
- If a step fails, show the error, say what it means, and offer the next option. Don't retry the
  same thing silently.
- The spec ([docs/spec.md](../spec.md)) and the platform READMEs are the source of truth. If a
  prompt here disagrees with them, follow them and say so.
- At the end, summarize what was installed and changed, and how to undo each item.
