import type { ReactNode } from "react";
import { EnginePage } from "./EnginePage";
import { MacPage } from "./MacPage";
import { MinecraftPage } from "./MinecraftPage";
import { WebPage } from "./WebPage";
import { WindowsPage } from "./WindowsPage";

/** The docs, in the repository's layout: the engine, then each platform it runs on. */
export const DOC_PAGES: { id: string; title: string; folder: string; summary: string; render(): ReactNode }[] = [
  { id: "engine", title: "Engine", folder: "engine/", summary: "Core logic, API, performance", render: () => <EnginePage /> },
  { id: "macos", title: "macOS", folder: "mac/", summary: "Input method and public API", render: () => <MacPage /> },
  { id: "minecraft", title: "Minecraft", folder: "minecraft/", summary: "Fabric mod", render: () => <MinecraftPage /> },
  { id: "windows", title: "Windows", folder: "windows/", summary: "TSF text input processor", render: () => <WindowsPage /> },
  { id: "web", title: "Web", folder: "web/", summary: "Playground and its engine API", render: () => <WebPage /> },
];

export function Docs({ page }: { page: (typeof DOC_PAGES)[number] }) {
  return (
    <div className="docs">
      <nav className="docs-nav" aria-label="Docs">
        <span className="docs-nav-label">Docs</span>
        <ul>
          {DOC_PAGES.map((p) => (
            <li key={p.id}>
              <a href={`#/docs/${p.id}`} aria-current={p.id === page.id ? "page" : undefined}>
                <span className="docs-nav-title">{p.title}</span>
                <span className="docs-nav-folder">{p.folder}</span>
              </a>
            </li>
          ))}
        </ul>
      </nav>
      <article className="doc">{page.render()}</article>
    </div>
  );
}

// Shared pieces for the pages.

export function Code({ children, lang }: { children: string; lang?: string }) {
  return (
    <pre className="code" data-lang={lang}>
      <code>{children.trim()}</code>
    </pre>
  );
}

/** A table of rows; the first column is set as code when `code` is on. */
export function Table({ head, rows, code = false }: { head: string[]; rows: ReactNode[][]; code?: boolean }) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            {head.map((h) => (
              <th key={h}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i}>
              {row.map((cell, j) => (
                <td key={j}>{j === 0 && code && typeof cell === "string" ? <code>{cell}</code> : cell}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function Source({ path, children }: { path: string; children?: ReactNode }) {
  return (
    <a href={`https://github.com/team-mirae/jis-korean/blob/main/${path}`} target="_blank" rel="noreferrer">
      {children ?? <code>{path}</code>}
    </a>
  );
}

export function PageHead({ title, folder, children }: { title: string; folder: string; children: ReactNode }) {
  return (
    <header className="doc-head">
      <span className="doc-folder">
        <Source path={folder}>{folder}</Source>
      </span>
      <h1>{title}</h1>
      <p className="lede">{children}</p>
    </header>
  );
}
