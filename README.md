# JIS Korean

A Korean 2-set (두벌식) input method for JIS keyboards, for macOS, Minecraft, Windows and iPadOS, with a
browser playground.

**Try it in your browser: [jis-korean.sjun.me](https://jis-korean.sjun.me)**

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/layout/jis-korean-layouts-dark.png">
  <img alt="The JIS Korean layouts. Mode A, for JIS keyboards: double consonants and compound vowels on the number row, Korean punctuation on the symbol keys, and 한글 / 한자 / 두벌식 / 전각 / 반각 on F6–F10. Mode B, for US ANSI keyboards (HHKB US and MacBook): the same number row and brackets, and every other key as US ANSI has it." src="docs/layout/jis-korean-layouts-light.png">
</picture>

Mode A is the original layout, built for JIS. Mode B carries it onto US ANSI keyboards
([spec §3.6](docs/spec.md#36-modes)); adding a mode of your own is described in
[docs/modes.md](docs/modes.md). The drawing is [docs/layout/jis-korean-layouts.html](docs/layout/jis-korean-layouts.html).

- Spec: [docs/spec.md](docs/spec.md)
- `engine/`: core engine (shared by all platforms)
- `mac/`: macOS input method
- `minecraft/`: Minecraft (Fabric mod)
- `windows/`: Windows input method (TSF)
- `ipados/`: iPadOS (stub)
- `web/`: browser playground, deployed at [jis-korean.sjun.me](https://jis-korean.sjun.me)

## Build order

Platforms are built in this order: **macOS → Minecraft → Windows → iPadOS**.

Android and iPhone aren't planned here; they'll be covered by a separate input method.
