import { PageHead } from "./Docs";

export function IPadOSPage() {
  return (
    <>
      <PageHead title="iPadOS" folder="ipados/">
        JIS Korean for iPad with a hardware keyboard. Planned: the folder is a stub, and iPadOS is last in the build order.
        Android and iPhone are out of scope.
      </PageHead>

      <h2>Reference keyboard</h2>
      <p>The iPadOS version is being built against a compact JIS keyboard:</p>
      <ul>
        <li>
          <strong>JIS layout:</strong> ¥ left of Backspace, <code>@</code> <code>[</code> right of P, <code>:</code>{" "}
          <code>]</code> right of L. All of JIS Korean's extra keys are there.
        </li>
        <li>
          <strong>Bottom row:</strong> Fn, 한, ⌥, ⌘, ABC, Space, かな, ⌘, ⌥, then the arrows. ABC and かな sit either side
          of Space, like 英数 and かな on a Mac JIS keyboard, and a dedicated 한 key sits left of ⌥.
        </li>
        <li>A pointing stick between G, H and B, and Control left of A.</li>
      </ul>

      <h2>API</h2>
      <p>None yet.</p>
    </>
  );
}
