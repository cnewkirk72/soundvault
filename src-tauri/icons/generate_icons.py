#!/usr/bin/env python3
"""
Generate Soundvault app icons from a procedural mark.

Output:
  32x32.png, 128x128.png, 128x128@2x.png, icon.png, icon.ico, icon.icns,
  plus Square*Logo.png variants for Windows store bundle target.

Run from src-tauri/icons/: `python3 generate_icons.py`.
"""

from __future__ import annotations

import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


HERE = Path(__file__).parent
SIZES_PNG = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 1024,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}

TOP = (122, 131, 245, 255)
BOTTOM = (63, 72, 163, 255)
HIGHLIGHT = (192, 199, 255, 255)
CYAN = (124, 213, 230, 255)


def make_icon(size: int) -> Image.Image:
    s = size
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    corner = max(int(s * 0.22), 1)
    base = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    for y in range(s):
        t = y / max(s - 1, 1)
        r = int(TOP[0] + (BOTTOM[0] - TOP[0]) * t)
        g = int(TOP[1] + (BOTTOM[1] - TOP[1]) * t)
        b = int(TOP[2] + (BOTTOM[2] - TOP[2]) * t)
        ImageDraw.Draw(base).line([(0, y), (s, y)], fill=(r, g, b, 255))

    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, s, s), corner, fill=255)
    img.paste(base, (0, 0), mask)

    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    g_draw = ImageDraw.Draw(glow)
    g_draw.ellipse((int(-s * 0.4), int(-s * 0.6), int(s * 1.4), int(s * 0.6)), fill=(255, 255, 255, 30))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=s * 0.05))
    img.alpha_composite(glow, (0, 0))

    cx, cy = s / 2, s / 2
    arc_color = (220, 225, 255, 230)
    arc_color_dim = (220, 225, 255, 140)
    arc_color_cyan = CYAN[:3] + (200,)

    line_w = max(int(s * 0.05), 2)
    dot_r = s * 0.06
    draw.ellipse((cx - dot_r, cy - dot_r, cx + dot_r, cy + dot_r), fill=(245, 246, 255, 255))
    arc_box_outer = (cx - s * 0.36, cy - s * 0.36, cx + s * 0.36, cy + s * 0.36)
    draw.arc(arc_box_outer, start=-65, end=65, fill=arc_color_dim, width=line_w)
    arc_box_mid = (cx - s * 0.26, cy - s * 0.26, cx + s * 0.26, cy + s * 0.26)
    draw.arc(arc_box_mid, start=-75, end=75, fill=arc_color, width=line_w)
    arc_box_inner = (cx - s * 0.17, cy - s * 0.17, cx + s * 0.17, cy + s * 0.17)
    draw.arc(arc_box_inner, start=-85, end=85, fill=arc_color_cyan, width=max(int(s * 0.04), 2))

    spec = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    sd = ImageDraw.Draw(spec)
    sd.ellipse((int(s * 0.12), int(s * 0.08), int(s * 0.5), int(s * 0.28)), fill=(255, 255, 255, 40))
    spec = spec.filter(ImageFilter.GaussianBlur(radius=s * 0.03))
    spec.putalpha(spec.split()[-1].point(lambda v: min(int(v * 1.3), 255)))
    img.alpha_composite(spec, (0, 0))

    return img


def save_pngs() -> dict:
    icons_at = {}
    for name, size in SIZES_PNG.items():
        if size not in icons_at:
            icons_at[size] = make_icon(size)
        icons_at[size].save(HERE / name, "PNG")
        print(f"wrote {name} ({size}x{size})")
    return icons_at


def save_ico(icons: dict):
    needed = [16, 32, 48, 64, 128, 256]
    rendered = {sz: icons.get(sz) or make_icon(sz) for sz in needed}
    rendered[needed[0]].save(
        HERE / "icon.ico",
        format="ICO",
        sizes=[(sz, sz) for sz in needed],
        append_images=[rendered[sz] for sz in needed[1:]],
    )
    print("wrote icon.ico (multi-res)")


def save_icns(icons: dict):
    pieces = [
        (b"ic04", make_icon(16)),
        (b"ic05", make_icon(32)),
        (b"ic07", icons.get(128) or make_icon(128)),
        (b"ic08", icons.get(256) or make_icon(256)),
        (b"ic09", make_icon(512)),
        (b"ic10", icons.get(1024) or make_icon(1024)),
        (b"ic11", make_icon(32)),
        (b"ic12", make_icon(64)),
        (b"ic13", icons.get(256) or make_icon(256)),
        (b"ic14", make_icon(512)),
    ]
    chunks = bytearray()
    for type_code, img in pieces:
        png_bytes = _to_png_bytes(img)
        size_field = struct.pack(">I", len(png_bytes) + 8)
        chunks += type_code + size_field + png_bytes
    total = 8 + len(chunks)
    header = b"icns" + struct.pack(">I", total)
    with open(HERE / "icon.icns", "wb") as f:
        f.write(header)
        f.write(chunks)
    print(f"wrote icon.icns ({total} bytes)")


def _to_png_bytes(img: Image.Image) -> bytes:
    from io import BytesIO
    buf = BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def main():
    HERE.mkdir(parents=True, exist_ok=True)
    icons = save_pngs()
    save_ico(icons)
    save_icns(icons)
    print("done.")


if __name__ == "__main__":
    main()
