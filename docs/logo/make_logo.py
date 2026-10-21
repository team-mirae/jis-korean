# SPDX-License-Identifier: MIT
# Copyright 2026 Mirae Research & Min Jun Sung. See docs/logo/README.md.
"""Draws the JIS Korean logo and the input mode badges as single-color SVGs.

Everything is one color; shapes are separated only by transparent borders: the S-curve between the
taegeuk's halves, and a border around each key, which also cuts through the taegeuk and through any
neighbouring key's border where they overlap. macOS draws input source icons as templates (one
color, recolored for light and dark menu bars), so these are built that way from the start.

The taegeuk follows the flag's construction: the dividing diameter on the flag's 2:3 slope, and
an S of two half-radius arcs. Key proportions are real keyboard units.

Usage: python3 -I make_logo.py   (writes taegeuk.svg, mode-a.svg, mode-b.svg, mode-iso.svg and
mode-a-qwerty.svg, mode-b-qwerty.svg and mode-cj-qwerty.svg here)
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


LETTER = 6  # the stroke of a drawn letter


LETTER_WIDTH = 23  # every letter's box, the same in every badge
LETTER_HEIGHT = 32
LETTER_GAP = 4


def letter_box(text):
    """The box (x, y, w, h, gap) for `text`, in the lower right corner, EDGE in from the canvas edge."""
    w = len(text) * LETTER_WIDTH + (len(text) - 1) * LETTER_GAP
    return EDGE - w, EDGE - LETTER_HEIGHT, w, LETTER_HEIGHT, LETTER_GAP


def letters(text, x, y, w, h, gap):
    """Letters in the style of a neo-grotesque (Helvetica) bold, as stroked centrelines in a box
    (x, y, w, h) of outer edges, gap apart: one stroke weight, square ends, flat-topped points, and
    C, G and S ending in horizontal cuts. Returns one (path, cuts, holes) per letter: cuts are
    rectangles (x, y, w, h) taken out of the stroke, which is how a round letter gets a horizontal
    terminal; holes are counters too small to show the disc through them (A's, B's), cleared with
    the letter's border instead of leaving slivers of the disc.
    Draws A B C D G H K M Q S U V Y Z."""
    n = len(text)
    lw = (w - (n - 1) * gap) / n
    half = LETTER / 2
    top, bottom = y + half, y + h - half
    mid = (top + bottom) / 2
    glyphs = []
    for i, ch in enumerate(text):
        lx = x + i * (lw + gap)
        left, right = lx + half, lx + lw - half
        cx = lx + lw / 2
        rx, ry = (right - left) / 2, (bottom - top) / 2  # an ellipse filling the box
        ellipse = (f"M{num(cx)} {num(top)}A{num(rx)} {num(ry)} 0 1 0 {num(cx)} {num(bottom)}"
                   f"A{num(rx)} {num(ry)} 0 1 0 {num(cx)} {num(top)}Z")
        far = lx + lw + 2  # past the letter's right edge, for cuts that open it on the right
        d, cuts, holes = "", [], ""
        if ch == "A":
            # The crossbar low, as in a grotesque.
            bar = top + (bottom - top) * 0.66
            t = (bar - top) / (bottom - top)
            d = (f"M{num(left)} {num(bottom)}L{num(cx)} {num(top)}L{num(right)} {num(bottom)}"
                 f"M{num(cx + (left - cx) * t)} {num(bar)}H{num(cx + (right - cx) * t)}")
            holes = f"M{num(cx)} {num(top)}L{num(cx + (left - cx) * t)} {num(bar)}H{num(cx + (right - cx) * t)}Z"
        elif ch == "B":
            # Straight top, middle and bottom; the lower bowl wider.
            r1, r2 = (mid - top) / 2, (bottom - mid) / 2
            upper, lower = right - r1 - 1.5, right - r2
            d = (f"M{num(left)} {num(bottom)}V{num(top)}H{num(upper)}A{num(r1)} {num(r1)} 0 0 1 {num(upper)} {num(mid)}"
                 f"H{num(left)}M{num(upper)} {num(mid)}H{num(lower)}A{num(r2)} {num(r2)} 0 0 1 {num(lower)} {num(bottom)}"
                 f"H{num(left)}")
            holes = (f"M{num(left)} {num(top)}H{num(upper)}A{num(r1)} {num(r1)} 0 0 1 {num(upper)} {num(mid)}H{num(left)}Z"
                     f"M{num(left)} {num(mid)}H{num(lower)}A{num(r2)} {num(r2)} 0 0 1 {num(lower)} {num(bottom)}H{num(left)}Z")
        elif ch == "C":
            # The ellipse, opened on the right by a band: the cuts are horizontal.
            d = ellipse
            cuts.append((cx, mid - ry * 0.48, far - cx, ry * 0.96))
        elif ch == "G":
            # C's upper cut; below it, a bar into the middle and the bowl closing up to it.
            d = ellipse + f"M{num(cx + rx * 0.08)} {num(mid + half * 0.5)}H{num(right)}V{num(bottom - ry * 0.4)}"
            cuts.append((cx, mid - ry * 0.48, far - cx, ry * 0.48))
        elif ch == "D":
            # The stem, and a bowl as wide as the letter.
            x1 = left + (right - left) * 0.3
            d = (f"M{num(left)} {num(top)}H{num(x1)}A{num(right - x1)} {num(ry)} 0 0 1 {num(x1)} {num(bottom)}"
                 f"H{num(left)}Z")
        elif ch == "H":
            d = (f"M{num(left)} {num(top)}V{num(bottom)}M{num(right)} {num(top)}V{num(bottom)}"
                 f"M{num(left)} {num(mid)}H{num(right)}")
        elif ch == "K":
            # The arm into the stem a little below the middle; the leg off the arm.
            join = mid + (bottom - top) * 0.08
            leg = (left + (right - left) * 0.38, join - (join - top) * 0.38)
            d = (f"M{num(left)} {num(top)}V{num(bottom)}M{num(right)} {num(top)}L{num(left)} {num(join)}"
                 f"M{num(leg[0])} {num(leg[1])}L{num(right)} {num(bottom)}")
        elif ch == "M":
            # Upright stems, and the middle point down on the baseline.
            d = f"M{num(left)} {num(bottom)}V{num(top)}L{num(cx)} {num(bottom)}L{num(right)} {num(top)}V{num(bottom)}"
        elif ch == "Q":
            # The ellipse, and a tail across its lower right.
            d = ellipse + f"M{num(cx + rx * 0.2)} {num(mid + ry * 0.45)}L{num(right + half * 0.3)} {num(bottom + half * 0.3)}"
        elif ch == "S":
            # Two bowls meeting in the middle, each ending in a horizontal cut: the upper one on
            # the right, the lower one on the left.
            ry1, ry2 = (mid - top) / 2, (bottom - mid) / 2
            d = (f"M{num(right)} {num(top + ry1)}A{num(rx)} {num(ry1)} 0 1 0 {num(cx)} {num(mid)}"
                 f"A{num(rx)} {num(ry2)} 0 1 1 {num(left)} {num(bottom - ry2)}")
            cuts.append((cx + rx * 0.2, top + ry1 * 0.55, far - cx, ry1 * 0.45 + half + 0.5))
            cuts.append((lx - 2, bottom - ry2 - half - 0.5, cx - rx * 0.2 - lx + 2, ry2 * 0.45 + half + 0.5))
        elif ch == "U":
            # Upright stems into a half ellipse.
            r = min(rx * 1.15, ry)
            d = (f"M{num(left)} {num(top)}V{num(bottom - r)}A{num(rx)} {num(r)} 0 0 0 {num(right)} {num(bottom - r)}"
                 f"V{num(top)}")
        elif ch == "V":
            d = f"M{num(left)} {num(top)}L{num(cx)} {num(bottom)}L{num(right)} {num(top)}"
        elif ch == "Y":
            # The arms meet just above the middle.
            meet = mid - (bottom - top) * 0.04
            d = (f"M{num(left)} {num(top)}L{num(cx)} {num(meet)}L{num(right)} {num(top)}"
                 f"M{num(cx)} {num(meet)}V{num(bottom)}")
        elif ch == "Z":
            d = f"M{num(left)} {num(top)}H{num(right)}L{num(left)} {num(bottom)}H{num(right)}"
        else:
            raise ValueError(f"no glyph for {ch}")
        glyphs.append((d, cuts, holes))
    return glyphs


def lettered(title, disc, letters):
    """The taegeuk with letters on top. Each letter cuts a transparent border half of BORDER wide
    into the disc around it, the same gap a key leaves (its border is half outside its edge), and
    the border follows the letter's cuts: it stops half of BORDER past each one."""
    cx, cy, r = disc
    full = '<rect width="100" height="100" fill="#fff"/>'
    mask = '<mask id="{0}" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="100">{1}</mask>'
    # Square ends and mitred corners, with sharp points flattened (miter limit 2), on the letters
    # and on their borders alike, so the gap is even all round.
    style = 'fill="none" stroke-linejoin="miter" stroke-miterlimit="2" stroke-linecap="square"'
    masks, halo, ink = "", "", ""
    for i, (d, cuts, holes) in enumerate(letters):
        gap = BORDER / 2
        if holes:
            halo += f'<path d="{holes}" fill="#000"/>'
        if cuts:
            # The letter loses its cuts; its border loses them less the gap above and below.
            letter_cut = "".join(f'<rect x="{num(x)}" y="{num(y)}" width="{num(w)}" height="{num(h)}" fill="#000"/>'
                                 for x, y, w, h in cuts)
            halo_cut = "".join(f'<rect x="{num(x)}" y="{num(y + gap)}" width="{num(w)}" height="{num(h - 2 * gap)}" fill="#000"/>'
                               for x, y, w, h in cuts if h > 2 * gap)
            masks += mask.format(f"l{i}", full + letter_cut) + mask.format(f"h{i}", full + halo_cut)
            halo += f'<g mask="url(#h{i})"><path d="{d}" {style} stroke="#000" stroke-width="{LETTER + BORDER}"/></g>'
            ink += f'<path d="{d}" mask="url(#l{i})"/>'
        else:
            halo += f'<path d="{d}" {style} stroke="#000" stroke-width="{LETTER + BORDER}"/>'
            ink += f'<path d="{d}"/>'
    s_cut = f'<path d="{s_curve(cx, cy, r)}" fill="none" stroke="#000" stroke-width="{S_GAP}"/>'
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">'
            "<!-- SPDX-License-Identifier: MIT. Copyright 2026 Mirae Research & Min Jun Sung. "
            "Drawn by docs/logo/make_logo.py; see docs/logo/README.md. -->"
            f"<title>{title}</title>"
            + masks
            + mask.format("disc", full + s_cut + halo)
            + f'<circle cx="{num(cx)}" cy="{num(cy)}" r="{num(r)}" fill="{INK}" mask="url(#disc)"/>'
            + f'<g {style} stroke="{INK}" stroke-width="{LETTER}">' + ink
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

    # A_QWERTY, JIS English: the taegeuk with ABC over its lower right.
    (out / "mode-a-qwerty.svg").write_text(
        lettered("JIS English (A_QWERTY)", (41, 41, 39), letters("ABC", *letter_box("ABC"))))

    # B_QWERTY and CJ_QWERTY, US and UK English: the same badge, with their country code.
    (out / "mode-b-qwerty.svg").write_text(
        lettered("US English (B_QWERTY)", (41, 41, 39), letters("US", *letter_box("US"))))
    (out / "mode-cj-qwerty.svg").write_text(
        lettered("UK English (CJ_QWERTY)", (41, 41, 39), letters("GB", *letter_box("GB"))))

    # The reserved English-layout modes (spec §3.6.1, ids 4-8): drawn ahead of their layouts.
    for name, title, text in [("mode-cj-qwertz", "CJ_QWERTZ", "QZ"), ("mode-cj-swiss-qz", "CJ_SWISS_QZ", "CH"),
                              ("mode-cj-azerty", "CJ_AZERTY", "AY"), ("mode-a-dvorak", "A_DVORAK", "DV"),
                              ("mode-a-colemak", "A_COLEMAK", "CMK")]:
        (out / f"{name}.svg").write_text(lettered(f"{title} (reserved)", (41, 41, 39), letters(text, *letter_box(text))))


if __name__ == "__main__":
    main()
