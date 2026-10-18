# web

Browser playground for JIS Korean. It runs the real engine (`/engine`, compiled to wasm32), so
it behaves exactly like the desktop input methods.

```sh
npm install
npm run dev     # builds the engine to wasm, then starts Vite
npm run build   # production build in dist/
```

Needs the Rust `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`).
