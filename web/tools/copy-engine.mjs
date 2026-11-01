// Copies the engine's wasm build and the data it loads into the site (`npm run wasm`). Plain Node,
// not `cp` / `mkdir -p`, so it runs the same in npm's shell on Windows (cmd.exe) as on macOS.
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");
const engine = join(web, "..", "engine");

const predict = [
  "ranker.bin",
  "learned.bin",
  "common-en.txt",
  "common-ko.txt",
  "english-nouns.txt",
  "LICENSE-wordnet.txt",
  "LICENSE-corpora.txt",
];

mkdirSync(join(web, "public", "predict"), { recursive: true });
copyFileSync(join(engine, "target", "wasm32-unknown-unknown", "release", "jis_korean.wasm"), join(web, "src", "engine.wasm"));
copyFileSync(join(engine, "dict", "hanja.tsv"), join(web, "public", "hanja.tsv"));
for (const f of predict) copyFileSync(join(engine, "predict", f), join(web, "public", "predict", f));
