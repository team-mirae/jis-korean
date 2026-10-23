import { useEffect, useRef, useState } from "react";
import { App } from "./App";
import { Docs, DOC_PAGES } from "./docs/Docs";
import { loadEngine, type Engine } from "./engine";

export const REPO = "https://github.com/team-mirae/jis-korean";

/** Each platform's release page, released separately (update the tags with each release). */
const DOWNLOADS = [
  { label: "Windows", tag: "v0.3.0-windows" },
  { label: "macOS", tag: "v0.3.0-macos" },
  { label: "Minecraft", tag: "v0.3.0-minecraft" },
];

/** Where the page is: `#/` the playground, `#/docs/<page>` a docs page. */
function useRoute(): string[] {
  const read = () => window.location.hash.replace(/^#\/?/, "").split("/").filter(Boolean);
  const [route, setRoute] = useState(read);
  useEffect(() => {
    const onChange = () => {
      setRoute(read());
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}

function GitHubMark() {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

/** The Download menu: a disclosure that also closes on Escape, an outside click or a pick. */
function DownloadMenu() {
  const menu = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const close = (e: Event) => {
      const el = menu.current;
      if (!el?.open) return;
      if (e instanceof KeyboardEvent ? e.key === "Escape" : !el.contains(e.target as Node)) el.open = false;
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", close);
    };
  }, []);
  return (
    <details className="download" ref={menu}>
      <summary>Download</summary>
      <ul>
        {DOWNLOADS.map(({ label, tag }) => (
          <li key={tag}>
            <a
              href={`${REPO}/releases/tag/${tag}`}
              target="_blank"
              rel="noreferrer"
              onClick={() => menu.current?.removeAttribute("open")}
            >
              {label}
            </a>
          </li>
        ))}
      </ul>
    </details>
  );
}

/** The engine for the playground, loaded once, the first time the playground is shown. */
let enginePromise: Promise<Engine> | null = null;

function Playground() {
  const [engine, setEngine] = useState<Engine | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    enginePromise ??= loadEngine();
    enginePromise.then(setEngine, (error) => {
      console.error(error);
      setFailed(true);
    });
  }, []);
  if (failed) return <p className="error">The engine failed to load.</p>;
  if (!engine) return <p className="loading">Loading the engine…</p>;
  return <App engine={engine} />;
}

export function Site() {
  const route = useRoute();
  const inDocs = route[0] === "docs";
  const page = DOC_PAGES.find((p) => p.id === route[1]) ?? DOC_PAGES[0];
  return (
    <>
      <header className="topbar">
        <div className="topbar-inner">
          <a className="brand" href="#/">
            JIS Korean
          </a>
          <nav aria-label="Site">
            <a href="#/" aria-current={inDocs ? undefined : "page"}>
              Playground
            </a>
            <a href={`#/docs/${DOC_PAGES[0].id}`} aria-current={inDocs ? "page" : undefined}>
              Docs
            </a>
          </nav>
          <span className="spacer" />
          <DownloadMenu />
          <a className="repo" href={REPO} target="_blank" rel="noreferrer" aria-label="JIS Korean on GitHub">
            <GitHubMark /> <span className="repo-label">GitHub</span>
          </a>
        </div>
      </header>
      {inDocs ? <Docs page={page} /> : <Playground />}
    </>
  );
}
