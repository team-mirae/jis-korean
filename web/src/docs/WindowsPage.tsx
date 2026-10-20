import { PageHead, Source } from "./Docs";

export function WindowsPage() {
  return (
    <>
      <PageHead title="Windows" folder="windows/">
        A Text Services Framework (TSF) text input processor around the engine. Planned: the folder holds no code yet, and
        Windows is third in the build order, after macOS and Minecraft.
      </PageHead>

      <h2>What it will be</h2>
      <ul>
        <li>
          <strong>A thin shell</strong>, like the macOS input method: Windows key events in, the engine's commit and
          composition text out, through the same C ABI (<Source path="engine/include/jis_korean.h" />).
        </li>
        <li>
          <strong>On the Japanese (106/109) keyboard setting</strong>, so the JIS keys' scan codes (¥ is <code>0x7D</code>,
          ろ is <code>0x73</code>) arrive intact.
        </li>
        <li>
          <strong>Inside every app's process</strong>, as every TSF input processor is. That's why the engine never panics
          and its C ABI catches any unwinding: a crash would take the app down with it.
        </li>
        <li>
          Shift+Z works as F10 here too: Windows browsers take F10 for their menu.
        </li>
      </ul>

      <h2>Known limits</h2>
      <p>Password fields and the secure desktop don't load text input processors.</p>

      <h2>API</h2>
      <p>None yet.</p>
    </>
  );
}
