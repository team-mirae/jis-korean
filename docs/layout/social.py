"""Renders the repository's social preview: the logo and Mode A's keys, 1280x640, light and dark.

    python3 -I docs/layout/social.py    # social-preview-{light,dark}.png next to this file

The keys come from jis-korean-layouts.html (its styles and its key tables), so the preview can't
drift from the drawing. Needs Google Chrome (CHROME overrides the path).
"""

import os
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
PAGE = HERE / "jis-korean-layouts.html"
LOGO = {"light": HERE.parent / "logo" / "taegeuk.svg", "dark": HERE.parent / "logo" / "taegeuk-dark.svg"}
CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
WIDTH, HEIGHT = 1280, 640

# The preview's own layout, after the drawing's styles: one row (logo and name) over the board.
STYLE = """
html, body { width: 1280px; height: 640px; overflow: hidden; }
body { display: grid; place-items: center; }
.social { display: grid; justify-items: center; gap: 34px; --u: 62px; }
.social .brand { display: flex; align-items: center; gap: 22px; }
.social .brand svg { width: 76px; height: 76px; }
.social .brand h1 { margin: 0; font-size: 58px; line-height: 1; letter-spacing: -0.02em; }
.social .brand p { margin: 8px 0 0; color: var(--muted); font-size: 21px; line-height: 1.3; }
.social .board { margin: 0; }
.hidden-boards { display: none; }
"""


def build(theme: str) -> str:
    page = PAGE.read_text()
    head = page[: page.index("</head>")]
    script = page[page.index("<script>") : page.index("</script>") + len("</script>")]
    logo = re.sub(r"<!--.*?-->|<title>.*?</title>", "", LOGO[theme].read_text(), flags=re.S)
    body = f"""<body>
<div class="social">
  <div class="brand">
    {logo}
    <div><h1>JIS Korean</h1><p>Korean 2-set (두벌식) for JIS keyboards</p></div>
  </div>
  <div class="board"><div class="plate" id="board-a"></div></div>
</div>
<div class="hidden-boards"><div id="board-b-hhkb"></div><div id="board-b-mac"></div><div id="board-c"></div></div>
{script}
</body>
</html>"""
    head = head.replace('<html lang="en">', f'<html lang="en" data-theme="{theme}">')
    return head + f"<style>{STYLE}</style>\n</head>\n" + body


def main() -> None:
    with tempfile.TemporaryDirectory() as work:
        for theme in ("light", "dark"):
            html = Path(work) / f"social-{theme}.html"
            html.write_text(build(theme))
            out = HERE / f"social-preview-{theme}.png"
            subprocess.run(
                [CHROME, "--headless", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
                 "--virtual-time-budget=5000", f"--window-size={WIDTH},{HEIGHT}", f"--screenshot={out}",
                 html.as_uri()],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            print(f"{out.name}: {WIDTH}x{HEIGHT}")


if __name__ == "__main__":
    main()
