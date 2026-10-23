# Set up JIS Korean for Minecraft

You come here from [start.md](start.md), or from the end of another platform's prompt, with the
session notes: official builds or their own, the operating system, and the keyboards.
[rules.md](rules.md) applies to every step: ask before each scan, and ask before each install or
change, one at a time.

Facts come from [minecraft/README.md](../../minecraft/README.md). Read it before you explain
anything, and follow it where this prompt is less precise. Skip start.md's remap question: the mod
reads the keys itself, and needs no switch key.

## 1. What it needs

- **Minecraft Java Edition 26.3** (26.3.x; not 26.4 or its snapshots, and nothing before 26.3);
- **Fabric Loader** 0.19.3+ and **Fabric API**;
- **Java 25**.

Ask which Minecraft version and launcher they use (the official launcher, Prism, MultiMC, a Fabric
profile...). A version outside that range can't load the mod: say so and stop there.

Read-only checks, each with permission or done by the user: the launcher's version list, and the
instance's `mods` folder for `fabric-api-*.jar`.

## 2. Get the jar

### Official builds

The newest release whose tag ends in `-minecraft` (start.md, "Official builds: the releases").
Download `jis-korean-<version>.jar` and check its SHA-256 against the release's `digest`. That's
the only check: the jar isn't signed.

### Build it yourself

The source is in a folder of its own (start.md section 6). Building needs, each asked about
separately (rules.md section 2):
1. **Java 25** (a JDK; Gradle also finds one through `gradle/gradle-daemon-jvm.properties`);
2. **Rust** through rustup, with the `wasm32-unknown-unknown` target (the engine runs as
   WebAssembly inside the game).

Then `./gradlew build` in `minecraft/` makes `build/libs/jis-korean-<version>.jar`.

## 3. Install

These are changes: ask before each one, or walk the user through it.
1. Fabric Loader for 26.3, if the instance doesn't have it (the Fabric installer or the launcher).
2. Fabric API into the instance's `mods` folder, if it isn't there.
3. The JIS Korean jar into the same `mods` folder. To update, remove the old `jis-korean-*.jar`
   first: Fabric won't start with two copies of the mod.

## 4. Using it

- It's always on in single-line text fields (chat, commands, anvils, search boxes), whatever the
  system's input source. Books and signs aren't supported yet.
- Every key goes into a staging area above the caret. Space, Enter or Tab commits whichever of
  Korean and English is more likely. **F6** forces Korean, **F7** forces English.
- **The mode follows JIS Korean's:** on macOS and Windows it reads the installed input method's mode
  (Mode A or B). Without JIS Korean installed, it's Mode A. With a US ANSI keyboard and no input
  method, offer to set up the platform's input method too (macos.md or windows.md), or say that
  Mode A's symbol keys are JIS's.

## 5. Check it works

Open chat and type `ㅎ ㅏ ㄴ ㄱ ㅡ ㄹ` then Space → 한글. Type `hello` then Space → hello.

## 6. Wrap up

List what was installed and where. To remove it, delete the jar from the `mods` folder.
