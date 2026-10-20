"""Packs the logo (mac/Resources/icons, drawn in docs/logo) into an .ico for the DLL. It holds the 16 px and 32 px PNGs as they are (PNG entries, Windows Vista+).

    python -I windows/tools/make-icons.py
"""
import pathlib
import struct

root = pathlib.Path(__file__).resolve().parents[2]
source = root / "mac" / "Resources" / "icons"
out = root / "windows" / "icons"
out.mkdir(exist_ok=True)

for name in ("taegeuk",):
    images = [(source / f"{name}.png").read_bytes(), (source / f"{name}@2x.png").read_bytes()]
    header = struct.pack("<HHH", 0, 1, len(images))
    entries = b""
    offset = 6 + 16 * len(images)
    for png in images:
        width, height = struct.unpack(">II", png[16:24])  # the IHDR chunk
        entries += struct.pack("<BBBBHHII", width % 256, height % 256, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
    (out / f"{name}.ico").write_bytes(header + entries + b"".join(images))
    print(out / f"{name}.ico")
