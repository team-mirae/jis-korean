#!/bin/sh
# Renders jis-korean-layouts.html to PNGs at 3x with headless Google Chrome, in light and dark.
#
#   ./render.sh                    the whole drawing: jis-korean-layouts-{light,dark}.png (the README's)
#   ./render.sh title mode-a ...   pieces: pieces/<piece>-{light,dark}.png
#   ./render.sh all                the whole drawing and every piece
#
# Pieces: title, mode-a, mode-b, mode-c, footer (the data-piece attributes in the page).
# CHROME overrides the browser path.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
chrome=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
page="$here/jis-korean-layouts.html"
pieces="title mode-a mode-b mode-c footer"
width=940
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# render <piece or ""> <output prefix>
render() {
  attrs=""
  [ -n "$1" ] && attrs=" data-piece=\"$1\""
  for theme in light dark; do
    sed "s/<html lang=\"en\">/<html lang=\"en\" data-theme=\"$theme\"$attrs>/" "$page" >"$work/page.html"
    height=$("$chrome" --headless --disable-gpu --window-size=$width,1200 --virtual-time-budget=5000 \
      --dump-dom "file://$work/page.html" 2>/dev/null | sed -n 's/.*data-height="\([0-9]*\)".*/\1/p' | head -1)
    [ -n "$height" ] || { echo "no height for ${1:-the whole drawing}" >&2; exit 1; }
    "$chrome" --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=3 \
      --virtual-time-budget=5000 --window-size=$width,"$height" \
      --screenshot="$2-$theme.png" "file://$work/page.html" >/dev/null 2>&1
  done
  echo "${1:-whole}: ${width}x$height"
}

[ $# -eq 0 ] && set -- whole
[ "$*" = all ] && set -- whole $pieces
for p in "$@"; do
  case " whole $pieces " in *" $p "*) ;; *) echo "unknown piece: $p (whole, all, $pieces)" >&2; exit 1 ;; esac
  if [ "$p" = whole ]; then
    render "" "$here/jis-korean-layouts"
  else
    mkdir -p "$here/pieces"
    render "$p" "$here/pieces/$p"
  fi
done
