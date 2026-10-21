# SPDX-License-Identifier: MIT
# Copyright 2026 Mirae Research & Min Jun Sung. See docs/logo/README.md.
"""Draws the JIS Korean logo and the input mode badges as single-color SVGs.

Everything is one color; shapes are separated only by transparent borders: the S-curve between the
taegeuk's halves, and a border around each key, which also cuts through the taegeuk and through any
neighbouring key's border where they overlap. macOS draws input source icons as templates (one
color, recolored for light and dark menu bars), so these are built that way from the start.

The taegeuk follows the flag's construction: the dividing diameter on the flag's 2:3 slope, and
an S of two half-radius arcs. Key proportions are real keyboard units.

Lettered badges need shapely (`pip install shapely`), which computes their shapes.

Usage: python3 make_logo.py   (writes taegeuk.svg, mode-a.svg, mode-b.svg, mode-iso.svg and
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
    C, G and S ending in horizontal cuts. Returns one (path, cuts) per letter: cuts are rectangles
    (x, y, w, h) taken out of the stroke, which is how a round letter gets a horizontal terminal.
    Draws A B C D G H K M Q S U V Y Z."""
    n = len(text)
    lw = (w - (n - 1) * gap) / n
    half = LETTER / 2
    top, bottom = y + half, y + h - half
    mid = (top + bottom) / 2
    glyphs = []

    def reach(p, q, ty):
        """The point where the line p → q crosses y = ty: a diagonal extended past the band, so
        trimming it gives a clean horizontal end."""
        t = (ty - p[1]) / (q[1] - p[1])
        return p[0] + (q[0] - p[0]) * t, ty

    def foot(p, ty, edge):
        """The point on y = ty for a diagonal from p whose outer edge (the side facing `edge`) meets
        y = ty exactly at x = edge, so the letter ends flush with its box."""
        xb = edge
        for _ in range(8):
            angle = math.atan2(abs(ty - p[1]), abs(xb - p[0]))
            xb = edge - math.copysign(half / math.sin(angle), edge - p[0])
        return xb, ty

    above, below = y - 2, y + h + 2  # past the cap line and the baseline
    for i, ch in enumerate(text):
        lx = x + i * (lw + gap)
        left, right = lx + half, lx + lw - half
        cx = lx + lw / 2
        rx, ry = (right - left) / 2, (bottom - top) / 2  # an ellipse filling the box
        ellipse = (f"M{num(cx)} {num(top)}A{num(rx)} {num(ry)} 0 1 0 {num(cx)} {num(bottom)}"
                   f"A{num(rx)} {num(ry)} 0 1 0 {num(cx)} {num(top)}Z")
        far = lx + lw + 2  # past the letter's right edge, for cuts that open it on the right
        d, cuts = "", []
        if ch == "A":
            # The crossbar low, as in a grotesque.
            bar = top + (bottom - top) * 0.66
            t = (bar - top) / (bottom - top)
            # The bar ends half a stroke inside each leg's centreline, so its square ends stay
            # inside the legs instead of poking out past them.
            l = reach((cx, top), foot((cx, top), y + h, lx), below)
            r_ = reach((cx, top), foot((cx, top), y + h, lx + lw), below)
            d = (f"M{num(l[0])} {num(l[1])}L{num(cx)} {num(top)}L{num(r_[0])} {num(r_[1])}"
                 f"M{num(cx + (left - cx) * t + half)} {num(bar)}H{num(cx + (right - cx) * t - half)}")
        elif ch == "B":
            # Straight top, middle and bottom; the lower bowl wider.
            r1, r2 = (mid - top) / 2, (bottom - mid) / 2
            upper, lower = right - r1 - 1.5, right - r2
            d = (f"M{num(left)} {num(bottom)}V{num(top)}H{num(upper)}A{num(r1)} {num(r1)} 0 0 1 {num(upper)} {num(mid)}"
                 f"H{num(left)}M{num(upper)} {num(mid)}H{num(lower)}A{num(r2)} {num(r2)} 0 0 1 {num(lower)} {num(bottom)}"
                 f"H{num(left)}")
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
            arm = reach((left, join), foot((left, join), y, lx + lw), above)
            toe = reach(leg, foot(leg, y + h, lx + lw), below)
            d = (f"M{num(left)} {num(top)}V{num(bottom)}M{num(arm[0])} {num(arm[1])}L{num(left)} {num(join)}"
                 f"M{num(leg[0])} {num(leg[1])}L{num(toe[0])} {num(toe[1])}")
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
            l = reach((cx, bottom), foot((cx, bottom), y, lx), above)
            r_ = reach((cx, bottom), foot((cx, bottom), y, lx + lw), above)
            d = f"M{num(l[0])} {num(l[1])}L{num(cx)} {num(bottom)}L{num(r_[0])} {num(r_[1])}"
        elif ch == "Y":
            # The arms meet just above the middle.
            meet = mid - (bottom - top) * 0.04
            l = reach((cx, meet), foot((cx, meet), y, lx), above)
            r_ = reach((cx, meet), foot((cx, meet), y, lx + lw), above)
            d = (f"M{num(l[0])} {num(l[1])}L{num(cx)} {num(meet)}L{num(r_[0])} {num(r_[1])}"
                 f"M{num(cx)} {num(meet)}V{num(bottom)}")
        elif ch == "Z":
            d = f"M{num(left)} {num(top)}H{num(right)}L{num(left)} {num(bottom)}H{num(right)}"
        else:
            raise ValueError(f"no glyph for {ch}")
        # Every letter is trimmed to its box: one cap line, one baseline, one width.
        cuts += [(lx - 12, y - 12, lw + 24, 12), (lx - 12, y + h, lw + 24, 12),
                 (lx - 12, y, 12, h), (lx + lw, y, 12, h)]
        glyphs.append((d, cuts))
    return glyphs


# --- Lettered badges are computed as geometry (shapely), not drawn with masks: each letter becomes
#     a filled outline, its transparent border is that outline grown by half of BORDER, and the
#     disc is what's left once the S and the borders are taken out, less any piece too small or
#     too thin to read as part of it (slivers between a letter and the disc's edge).

ARC_STEPS = 64  # points per half ellipse when flattening arcs
MIN_PIECE_AREA = 20  # a leftover piece of the disc smaller than this (square units) is dropped
MIN_PIECE_THICKNESS = 1.6  # ... and so is one thinner than this (twice its area over its perimeter)


def flatten(d):
    """An SVG path of M L H V A Z (arcs unrotated) as subpaths: (points, closed)."""
    import re
    tokens = re.findall(r"[MLHVAZ]|-?[0-9.]+", d)
    subpaths, points, pos, i = [], [], (0.0, 0.0), 0
    cmd = None

    def take(n):
        nonlocal i
        values = [float(t) for t in tokens[i:i + n]]
        i += n
        return values

    while i < len(tokens):
        if tokens[i].isalpha():
            cmd = tokens[i]
            i += 1
        if cmd == "M":
            if len(points) > 1:
                subpaths.append((points, False))
            pos = tuple(take(2))
            points = [pos]
            cmd = "L"
        elif cmd == "L":
            pos = tuple(take(2))
            points.append(pos)
        elif cmd == "H":
            pos = (take(1)[0], pos[1])
            points.append(pos)
        elif cmd == "V":
            pos = (pos[0], take(1)[0])
            points.append(pos)
        elif cmd == "A":
            rx, ry, _, large, sweep, x, y = take(7)
            points.extend(arc_points(pos, (x, y), rx, ry, large, sweep))
            pos = (x, y)
        elif cmd == "Z":
            subpaths.append((points, True))
            points = [points[0]]
            pos = points[0]
            cmd = None
    if len(points) > 1:
        subpaths.append((points, False))
    return subpaths


def arc_points(p1, p2, rx, ry, large, sweep):
    """Points along an unrotated SVG elliptical arc from p1 to p2 (the endpoint conversion of the
    SVG spec, with the radii scaled up if they're too small)."""
    x1p, y1p = (p1[0] - p2[0]) / 2, (p1[1] - p2[1]) / 2
    scale = (x1p / rx) ** 2 + (y1p / ry) ** 2
    if scale > 1:
        rx, ry = rx * math.sqrt(scale), ry * math.sqrt(scale)
    num_ = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p
    den = rx * rx * y1p * y1p + ry * ry * x1p * x1p
    k = math.sqrt(max(0.0, num_ / den)) * (-1 if large == sweep else 1)
    cxp, cyp = k * rx * y1p / ry, -k * ry * x1p / rx
    cx, cy = cxp + (p1[0] + p2[0]) / 2, cyp + (p1[1] + p2[1]) / 2
    a1 = math.atan2((y1p - cyp) / ry, (x1p - cxp) / rx)
    a2 = math.atan2((-y1p - cyp) / ry, (-x1p - cxp) / rx)
    delta = a2 - a1
    if sweep and delta < 0:
        delta += 2 * math.pi
    elif not sweep and delta > 0:
        delta -= 2 * math.pi
    steps = max(2, int(abs(delta) / math.pi * ARC_STEPS))
    return [(cx + rx * math.cos(a1 + delta * t / steps), cy + ry * math.sin(a1 + delta * t / steps))
            for t in range(1, steps + 1)]


def stroke(d, width, cap="square", join="mitre", mitre=2.0):
    """The outline of path `d` stroked `width` wide, as SVG would draw it."""
    from shapely.geometry import LinearRing, LineString
    from shapely.ops import unary_union
    parts = []
    for points, closed in flatten(d):
        line = LinearRing(points) if closed else LineString(points)
        parts.append(line.buffer(width / 2, cap_style=cap if not closed else "flat", join_style=join,
                                 mitre_limit=mitre, quad_segs=16))
    return unary_union(parts)


def svg_path(geometry):
    """A filled shapely geometry as one SVG path (even-odd)."""
    polygons = getattr(geometry, "geoms", [geometry])
    d = ""
    for polygon in polygons:
        if polygon.is_empty:
            continue
        for ring in [polygon.exterior, *polygon.interiors]:
            coords = list(ring.coords)[:-1]
            d += "M" + "L".join(f"{num(x)} {num(y)}" for x, y in coords) + "Z"
    return d


def lettered(title, disc, letters):
    """The taegeuk with letters on top, as filled shapes: each letter's outline, grown by half of
    BORDER (a key's gap), is taken out of the disc; then any leftover piece of the disc that's too
    small or too thin to read (MIN_PIECE_*) goes too, so no jagged slivers are left."""
    from shapely.geometry import Point, box
    from shapely.ops import unary_union
    cx, cy, r = disc
    inks = []
    for d, cuts in letters:
        ink = stroke(d, LETTER)
        for x, y, w, h in cuts:
            ink = ink.difference(box(x, y, x + w, y + h))
        inks.append(ink)
    ink = unary_union(inks)
    border = ink.buffer(BORDER / 2, join_style="round", quad_segs=16)
    s_band = stroke(s_curve(cx, cy, r), S_GAP, cap="flat", join="round")
    shape = Point(cx, cy).buffer(r, quad_segs=64).difference(s_band).difference(border)
    pieces = [p for p in getattr(shape, "geoms", [shape])
              if p.area >= MIN_PIECE_AREA and 2 * p.area / p.length >= MIN_PIECE_THICKNESS]
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">'
            "<!-- SPDX-License-Identifier: MIT. Copyright 2026 Mirae Research & Min Jun Sung. "
            "Drawn by docs/logo/make_logo.py; see docs/logo/README.md. -->"
            f"<title>{title}</title>"
            f'<path fill="{INK}" fill-rule="evenodd" d="{svg_path(unary_union(pieces))}"/>'
            f'<path fill="{INK}" fill-rule="evenodd" d="{svg_path(ink)}"/>'
            "</svg>\n")


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
