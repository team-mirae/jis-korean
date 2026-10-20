# SPDX-License-Identifier: MIT
# Copyright 2026 Mirae Research & Min Jun Sung. See docs/logo/README.md.
"""Draws the JIS Korean logo and the input mode badges as single-color SVGs.

Everything is one color; shapes are separated only by transparent borders: the S-curve between the
taegeuk's halves, and a border around each key, which also cuts through the taegeuk and through any
neighbouring key's border where they overlap. macOS draws input source icons as templates (one
color, recolored for light and dark menu bars), so these are built that way from the start.

The taegeuk follows the flag's construction: the dividing diameter on the flag's 2:3 slope, and
an S of two half-radius arcs. Key proportions are real keyboard units.

Usage: python3 -I make_logo.py   (writes taegeuk.svg, mode-a.svg, mode-b.svg, mode-iso.svg here)
"""

import math
from pathlib import Path

INK = "#000000"
S_GAP = 6  # the transparent S between the halves
BORDER = 6  # the transparent border around each key (half outside, half inside its edge)
CORNER = 7  # key corner radius (the border takes half its width off, leaving about 4)
EDGE = 98  # keys stop here, so their borders stay inside the 100-unit canvas


def num(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def s_curve(cx, cy, r):
    """The taegeuk's dividing S, as an open path."""
    ux, uy = 3 / math.sqrt(13), 2 / math.sqrt(13)
    p1, p2 = (cx - r * ux, cy - r * uy), (cx + r * ux, cy + r * uy)
    h = r / 2
    return (f"M{num(p1[0])} {num(p1[1])}A{num(h)} {num(h)} 0 0 0 {num(cx)} {num(cy)}"
            f"A{num(h)} {num(h)} 0 0 1 {num(p2[0])} {num(p2[1])}")


def rounded(points, radius=CORNER):
    """A polygon with rounded corners."""
    d = ""
    for i, p in enumerate(points):
        before, after = points[i - 1], points[(i + 1) % len(points)]

        def toward(b):
            length = math.hypot(b[0] - p[0], b[1] - p[1])
            t = min(radius, length / 2) / length
            return p[0] + (b[0] - p[0]) * t, p[1] + (b[1] - p[1]) * t

        s, e = toward(before), toward(after)
        d += ("M" if i == 0 else "L") + f"{num(s[0])} {num(s[1])}Q{num(p[0])} {num(p[1])} {num(e[0])} {num(e[1])}"
    return d + "Z"


def rect(x, y, w, h, radius=CORNER):
    return rounded([(x, y), (x + w, y), (x + w, y + h), (x, y + h)], radius)


def badge(title, disc, keys, hollow=()):
    """disc: (cx, cy, r). keys: key outlines (paths). hollow: (x, y, w, h) of keys drawn as outlines.

    The S cuts the disc only; the key borders cut everything, the disc and each other included.
    """
    cx, cy, r = disc
    full = '<rect width="100" height="100" fill="#fff"/>'
    s_cut = f'<path d="{s_curve(cx, cy, r)}" fill="none" stroke="#000" stroke-width="{S_GAP}"/>'
    borders = "".join(f'<path d="{k}" fill="none" stroke="#000" stroke-width="{BORDER}"/>' for k in keys)
    # A hollow key keeps a ring as thick as the border; its inside is cut away.
    borders += "".join(
        f'<path d="{rect(x + BORDER, y + BORDER, w - 2 * BORDER, h - 2 * BORDER, 2)}" fill="#000"/>' for x, y, w, h in hollow
    )
    mask = '<mask id="{0}" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="100">{1}</mask>'
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">'
            "<!-- SPDX-License-Identifier: MIT. Copyright 2026 Mirae Research & Min Jun Sung. "
            "Drawn by docs/logo/make_logo.py; see docs/logo/README.md. -->"
            f"<title>{title}</title>"
            + mask.format("s", full + s_cut)
            + mask.format("borders", full + borders)
            + f'<g fill="{INK}" mask="url(#borders)">'
            f'<circle cx="{num(cx)}" cy="{num(cy)}" r="{num(r)}" mask="url(#s)"/>'
            + "".join(f'<path d="{k}"/>' for k in keys)
            + "</g></svg>\n")


def main():
    out = Path(__file__).parent

    # The logo alone.
    (out / "taegeuk.svg").write_text(badge("Taegeuk", (50, 50, 48), []))

    # Mode A, JIS: the ISO-style Enter (1.5 units on the upper row, 1.25 on the lower, two rows).
    u = 29
    x, y = EDGE - 1.5 * u, EDGE - 2 * u
    enter = rounded([(x, y), (EDGE, y), (EDGE, EDGE), (x + 0.25 * u, EDGE), (x + 0.25 * u, y + u), (x, y + u)])
    (out / "mode-a.svg").write_text(badge("JIS Korean, Mode A (JIS)", (41, 41, 39), [enter]))

    # Mode B, US ANSI: backslash (1.5 units) above a one-row Enter (2.25 units), clearly apart.
    u, gap = 27, 5
    backslash = rect(EDGE - 1.5 * u, EDGE - 2 * u - gap, 1.5 * u, u)
    enter = rect(EDGE - 2.25 * u, EDGE - u, 2.25 * u, u)
    (out / "mode-b.svg").write_text(badge("JIS Korean, Mode B (US ANSI)", (41, 41, 39), [backslash, enter]))

    # Future ISO mode: Caps Lock over a short left Shift, the ISO extra key (hollow) and Z.
    u, gap, left = 24, 4, 2
    caps = (left, EDGE - 2 * u - gap, 1.75 * u, u)
    shift = (left, EDGE - u, 1.25 * u, u)
    extra = (left + 1.25 * u + gap, EDGE - u, u, u)
    z = (left + 2.25 * u + 2 * gap, EDGE - u, u, u)
    keys = [rect(*k) for k in (caps, shift, extra, z)]
    (out / "mode-iso.svg").write_text(badge("JIS Korean, future ISO mode", (59, 41, 39), keys, hollow=[extra]))


if __name__ == "__main__":
    main()
