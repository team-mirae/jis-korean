# web

Live at **https://jis-korean.sjun.me** (Vercel project `jis-korean`).

Browser playground for JIS Korean. It runs the real engine (`/engine`, compiled to wasm32), so
it behaves exactly like the desktop input methods.

```sh
npm install
npm run dev     # builds the engine to wasm, then starts Vite
npm run build   # production build in dist/
```

Needs the Rust `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).

## Deploy

Vercel's build machines have no Rust, so the site is built locally and uploaded prebuilt:

```sh
vercel pull --yes --environment=production
vercel build --prod          # runs npm run build: engine → wasm, then Vite
vercel deploy --prebuilt --prod
```

## Key-press counts

The live comparison under the typing area runs the engine's planner (`engine/src/plan.rs`, exported
as `jk_plan`) on whatever is in the box, or on `public/sample.txt` when it's empty. Shift counts as
a press, and a character a layout has no key for counts as 3 on that layout.

The table of well-known texts is precomputed into `src/stats.json` by:

```sh
python3 -I web/tools/build_stats.py   # from the repo root; fetches from Korean Wikisource, needs cargo
```

It stores only the counts and the exact source revisions, not the texts.
