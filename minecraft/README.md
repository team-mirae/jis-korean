# minecraft

**JIS Korean for Minecraft**: a Fabric mod (client only) that types Korean in Minecraft text
fields with the JIS Korean layout, and decides word by word whether you meant Korean or English.

Minecraft **[26.3, 26.4)** (`>=26.3 <26.4-`, which also excludes 26.4 snapshots). 26.3 moved
input from GLFW to SDL, and this mod reads SDL scancodes, so no earlier version can load it.
Requires Fabric Loader 0.19.3+ (the same as Fabric API 0.163.0+26.3), Fabric API and Java 25.

## How it works

Every key you type goes into a **staging area** (shown in a small window above the caret), and
the staging area is read two ways from the same keys:

- **Korean**: what JIS Korean composes (`gksrmf` → 한글), from the same engine as the macOS input
  method and the [web playground](https://jis-korean.sjun.me).
- **English**: what the keys type on a plain JIS English layout (`gksrmf`).

**Space, Enter or Tab** commits the more likely reading. Space adds a space, Enter then sends the
line, and Tab only commits. Minecraft's own suggestion line isn't used, so chat command
suggestions keep working.

| Key | Effect |
| --- | --- |
| Space / Enter / Tab | Commit the ranked reading |
| **F6** | Force Korean: the engine's F6, which commits (dropping a plain implied ㅇ) |
| **F7** | Force English (in place of the engine's hanja) |
| F8 / Shift+F8, F9, F10 | As in JIS Korean: 2-set escape and back, full- and half-width |
| Backspace | Edits the staging area by jamo, or deletes in the field when it's empty |
| Escape | Drops the staging area, then closes the screen |
| F18 / 英数 | Mod on / off (rebindable under Controls → JIS Korean) |

F6 and F7 choices are remembered for the rest of the session: force `dkssud` to English once and
it stays English.

### Ranking

`me.sjun.jiskorean.rank.Ranker` compares two key-trigram models on the keys you pressed: one
trained on English spelling, one on the keys JIS Korean users press for Korean words (with the
implied ㅇ and the double press). The models are built by `tools/build_ranker.py` from
redistributable data:

- English: [dwyl/english-words](https://github.com/dwyl/english-words) (Unlicense) and
  `common-en.txt`
- Korean: the readings in `engine/dict/hanja.tsv` (libhangul, BSD-3-Clause) and `common-ko.txt`

Held-out accuracy (from the build script's report):

| Test set | Accuracy |
| --- | --- |
| English words | 99.5% |
| Korean words (JIS Korean typing) | 99.2% |
| Real Korean text, word by word (Samsung Electronics' "사업의 내용", not trained on) | about 96% |

Rules on top of the model:
- **Shift on a key that Korean never shifts** (anything but Q W E R T O P) means English.
- **Exact matches** in the common-word lists lean their way.
- **Leftover bare jamo** count against Korean, except chat words like ㅋㅋ.
- **A `/command` name** is always English. Its arguments are ranked as usual.
- **The previous word's language** tips close calls.

## Build and test

```sh
./gradlew build               # build/libs/jis-korean-0.1.0.jar (engine, model and Chicory inside)
./gradlew test                # staging, ranking and engine tests (no game)
./gradlew runClientGameTest   # boots the real client, opens chat and types through the mod
./gradlew runClient           # play with it
```

The engine is `../engine`, compiled to WebAssembly during the build (needs Rust with the
`wasm32-unknown-unknown` target) and run on [Chicory](https://chicory.dev), a pure-Java Wasm
runtime: one engine for every OS, no native libraries. Gradle runs on Java 25
(`gradle/gradle-daemon-jvm.properties`). To rebuild the ranker model:

```sh
python3 -I tools/build_ranker.py words_alpha.txt [--eval-korean <Korean text>]
```

## Notes

- **Keep the OS input source on English** (ABC) while playing. The mod does the Korean itself, and
  an OS input method would compose on top of it.
- **Karabiner ろ rule:** on Macs where a Karabiner rule turns ろ into Option+¥ (as in
  hhkb-ekj-trilingual), the mod reads Option+¥ back as ろ.
- **Supported fields:** single-line text fields (`EditBox`): chat, commands, anvils, command
  blocks, search boxes. Books and signs use other widgets and aren't supported yet.
