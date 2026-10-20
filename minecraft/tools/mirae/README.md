# Mirae server vocabulary

Game vocabulary for the Minecraft mod's Korean-or-English ranker, from 미래서버 (Mirae), a live
Minecraft server run in 2025 on Paper 1.21.8 with the Mirae plugin
(https://github.com/themrsung/Mirae, MIT, Copyright (c) 2025-2026 Min Jun Sung). It's Minecraft-only:
none of it goes into the engine's shared word lists.

| File | What | Committed |
|---|---|---|
| `harvest.py` | Derives word lists from the server's chat logs | yes |
| `commands.py` | Lists the plugin's commands and aliases from its source | yes |
| `commands.txt` | Every command name and alias, verbatim (public, MIT source) | yes |
| `data/logs/` | The server logs | **never** (gitignored) |
| `data/vocab-ko.txt`, `data/vocab-en.txt` | Words derived from public chat, with counts | **never** (gitignored) |

## The chat logs are private

The logs come from the server's own log files (the `logs/` folder of the Mirae server). They contain
players' usernames and conversations, so they and everything derived from them stay in `data/`,
which is gitignored. To rebuild the word lists, unzip the logs into `data/logs/` and run
`python3 -I harvest.py`.

What `harvest.py` reads (worked out from the plugin's source: `PlayerListener.renderChat` logs public
and local chat to the console; `DirectMessage.asShownToConsole` logs DMs):

- **Kept:** public chat and local chat (`[지역] `), as the plugin logs them from the async chat
  thread: optional `[지역] `, optional `:icon:` tokens, the sender's display name, ` : `, the message.
- **Never read:** direct messages and replies (`[sender -> recipient] …`, server thread), anything
  typed as a command (`issued server command`), muted players' messages (`[Muted] `, seen only by
  the sender), and plugin notices on the chat thread (GriefPrevention).

How names are kept out:

1. Every name the logs show is collected first: account names (joins, leaves, logins, commands, shop
   and plugin lines, DMs), display names in chat, and nicknames set with `/nick` and its aliases.
2. A word that contains a name, or is a fragment of one, is dropped.
3. Nicknames used only in conversation are caught by how Korean addresses people: a word also seen
   with ~님 ~씨 ~형 ~쌤 ~아 ~야 is taken for a person and dropped in every form.
4. Only single words are kept, never messages, and only words used at least twice by at least two
   different people.
5. The script prints counts only.

An audit after the last run found no kept word equal to, containing, inside, or one edit away from
any collected name.

English words that are really Korean typed with the keyboard in English mode (`zzz` for ㅋㅋㅋ, `dk`
for 아, `dlrj` for 이거) are left out of the English list.

## Commands

`commands.py` reads every command the plugin registers (`Commands.getCommands()`) with its aliases
(`setAliases(List.of(…))`), verbatim:

```sh
git clone --depth 1 https://github.com/themrsung/Mirae.git /tmp/Mirae
python3 -I commands.py /tmp/Mirae
```

`commands.txt` records the commit it was generated from. Korean aliases (`/귓말`, `/지역`, `/스폰`)
matter to the mod: a command name is otherwise always ranked English.
