# JIS Korean for Minecraft: attributions

JIS Korean for Minecraft is a Fabric mod that types Korean in Minecraft with the JIS Korean layout.
Source, releases and documentation: https://github.com/team-mirae/jis-korean

This file travels with the jar, so the credits stay with it wherever it's shared. The license
texts named below are in the jar's root, next to this file.

## JIS Korean

Copyright 2026 Mirae Research & Min Jun Sung. MIT License: `LICENSE_jis-korean`.

The engine (`assets/jiskorean/engine.wasm`) is JIS Korean's own, written in Rust and compiled to
WebAssembly. It uses no third-party crates.

## Bundled libraries

- **Chicory** 1.7.5 (`META-INF/jars/runtime-1.7.5.jar`, `META-INF/jars/wasm-1.7.5.jar`), the
  pure-Java WebAssembly runtime that runs the engine. https://github.com/dylibso/chicory
  Apache License 2.0: `LICENSE-chicory.txt`. Not modified.

## Bundled data

The word lists (`assets/jiskorean/common-en.txt`, `common-ko.txt`) and the ranker model
(`assets/jiskorean/ranker.bin`) are derived from the sources below: word counts and readings only,
no sentences are shipped.

- **libhangul**'s hanja list (`data/hanja/hanja.txt`), Copyright (c) 2005, 2006 Choe Hwanjin.
  BSD-3-Clause: `LICENSE-libhangul-hanja.txt`. https://github.com/libhangul/libhangul
- **The Unicode Unihan database** (`Unihan_Readings.txt`), readings in the hanja list. Unicode
  License v3: `LICENSE-unicode.txt`. https://www.unicode.org/reports/tr38/
- **Princeton WordNet 3.0**, which weights English nouns. WordNet License: `LICENSE-wordnet.txt`.
  https://wordnet.princeton.edu
- **dwyl/english-words** (`words_alpha.txt`), the English spelling model. The Unlicense.
  https://github.com/dwyl/english-words
- **Colloquial corpora and business term lists**, each with its license in `LICENSE-corpora.txt`:
  - NSMC, the Naver sentiment movie corpus v1.0, by Lucy Park (CC0 1.0).
    https://github.com/e9t/nsmc
  - Chatbot_data by songys (MIT). https://github.com/songys/Chatbot_data
  - Tatoeba's English sentences (CC BY 2.0 FR). https://tatoeba.org
  - Taskmaster-1 (2019), Google Research, by Bill Byrne et al. (CC BY 4.0).
    https://github.com/google-research-datasets/Taskmaster
  - 행정안전부 공통표준용어 and 공통표준단어, 공공데이터포털 (no restriction on use).
    https://www.data.go.kr
  - FIBO, the Financial Industry Business Ontology, by the EDM Council (MIT).
    https://github.com/edmcouncil/fibo
- **The Mirae plugin's commands and aliases** (`assets/jiskorean/mirae-commands.txt`), from
  https://github.com/themrsung/Mirae, Copyright (c) 2025-2026 Min Jun Sung. MIT License:
  `LICENSE-mirae.txt`.
- **Words from 미래서버 (Mirae)'s public chat** (`assets/jiskorean/mirae-vocab-ko.txt`,
  `mirae-vocab-en.txt`), only in jars built with them. The official releases don't have them.
  Single words, with names left out (`tools/mirae/README.md` in the source).

## Required, not bundled

The mod runs on, and needs, these. None of them is inside this jar.

- **Minecraft** Java Edition 26.3, by Mojang Studios. JIS Korean is not an official Minecraft
  product and is not approved by or associated with Mojang or Microsoft.
- **Fabric Loader** and **Fabric API**, by FabricMC (Apache License 2.0).
  https://fabricmc.net
- Java 25.

## Credits

- Keyboard setups and research: **keyboard-troubleshooting** by Min Jun Sung (CC0-1.0),
  https://github.com/themrsung/keyboard-troubleshooting
- Game vocabulary: the **미래서버 (Mirae)** Minecraft server and its plugin.
