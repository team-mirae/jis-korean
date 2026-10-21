#!/bin/sh
# SPDX-License-Identifier: MIT
# Renders each badge SVG to the PNGs the macOS input method ships (mac/Resources/icons/):
# <name>.png (16 px) and <name>@2x.png (32 px), transparent, one color.
#
# The SVGs use masks, which macOS's own SVG support (NSImage) doesn't fully draw, so they're rendered
# with headless Chrome, which draws them as browsers do. Headless Chrome won't go below a 500 px
# window, so each is drawn at 1024 px and scaled down by downscale.swift, which halves in steps with
# high-quality filtering so curves keep smooth edges.
#
# Usage: sh render.sh   (after python3 -I make_logo.py; needs Google Chrome)
set -eu
here=$(cd "$(dirname "$0")" && pwd)
out="$here/../../mac/Resources/icons"
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

for name in taegeuk mode-a mode-b mode-iso mode-a-qwerty mode-b-qwerty mode-c-qwerty; do
  cp "$here/$name.svg" "$work/$name.svg"
  printf '<!doctype html><style>html,body{margin:0;background:transparent}img{display:block}</style><img src="%s.svg" width="1024" height="1024">' \
    "$name" >"$work/$name.html"
  "$chrome" --headless --disable-gpu --hide-scrollbars --allow-file-access-from-files \
    --default-background-color=00000000 --window-size=1024,1024 \
    --screenshot="$work/$name-1024.png" "file://$work/$name.html" >/dev/null 2>&1
  swift "$here/downscale.swift" "$work/$name-1024.png" "$out/$name@2x.png" 32
  swift "$here/downscale.swift" "$work/$name-1024.png" "$out/$name.png" 16
  echo "$out/$name.png, $name@2x.png"
done
