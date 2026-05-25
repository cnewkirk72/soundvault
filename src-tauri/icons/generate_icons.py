#!/usr/bin/env python3
"""
Generate Soundvault app icons.

Design: a folder outline in soft silver with a faint violet glow, an audio
circle inset, and a brightly glowing purple waveform inside it. Dark near-black
rounded-square background.

Output:
  32x32.png, 128x128.png, 128x128@2x.png, icon.png (1024), icon.ico, icon.icns,
  plus Square*Logo.png variants for Windows store bundle target.

Run from src-tauri/icons/: `python3 generate_icons.py`.
"""

from __future__ import annotations

import struct
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

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

# --- Palette -------------------------------------------------------------

BG_TOP = (22, 22, 28)
BG_BOTTOM = (8, 8, 10)

SILVER = (225, 226, 235)
SILVER_HIGHLIGHT = (245, 246, 252)

PURPLE_BAR = (184, 122, 255)
PURPLE_BAR_HOT = (212, 168, 255)
PURPLE_GLOW = (138, 72, 255)


# --- Helpers -------------------------------------------------------------

def make_glow(alpha_mask, color, blur, opacity=1.0):
    s = alpha_mask.size
    glow = Image.new("RGBA", s, color + (0,))
    solid = Image.new("RGBA", s, color + (255,))
    glow = Image.composite(solid, glow, alpha_mask)
    glow = glow.filter(ImageFilter.GaussianBlur(radius=blur))
    if opacity != 1.0:
        a = glow.split()[-1].point(lambda v: int(v * opacity))
        glow.putalpha(a)
    return glow


def fill_from_mask(mask, color):
    rgba = Image.new("RGBA", mask.size, color + (255,))
    rgba.putalpha(mask)
    return rgba


def gradient_bg(size):
    img = Image.new("RGB", (size, size), BG_TOP)
    d = ImageDraw.Draw(img)
    for y in range(size):
        t = y / max(size - 1, 1)
        r = int(BG_TOP[0] + (BG_BOTTOM[0] - BG_TOP[0]) * t)
        g = int(BG_TOP[1] + (BG_BOTTOM[1] - BG_TOP[1]) * t)
        b = int(BG_TOP[2] + (BG_BOTTOM[2] - BG_TOP[2]) * t)
        d.line([(0, y), (size, y)], fill=(r, g, b))
    return img.convert("RGBA")


def rounded_square_mask(size, corner_frac=0.22):
    mask = Image.new("L", (size, size), 0)
    corner = max(int(size * corner_frac), 1)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size, size), corner, fill=255)
    return mask


# --- Shapes --------------------------------------------------------------

def folder_outline_mask(s, stroke):
    margin_x = s * 0.18
    folder_top = s * 0.26
    folder_bottom = s * 0.82
    folder_left = margin_x
    folder_right = s - margin_x
    tab_right = folder_left + (folder_right - folder_left) * 0.42
    tab_height = s * 0.05
    body_top = folder_top + tab_height

    r_out = max(int(s * 0.038), 2)
    r_in = max(r_out - stroke, 1)

    outer = Image.new("L", (s, s), 0)
    od = ImageDraw.Draw(outer)
    od.rounded_rectangle(
        (folder_left, body_top, folder_right, folder_bottom),
        radius=r_out, fill=255,
    )
    od.rounded_rectangle(
        (folder_left, folder_top, tab_right, body_top + r_out),
        radius=r_out, fill=255,
    )

    inner = Image.new("L", (s, s), 0)
    idr = ImageDraw.Draw(inner)
    idr.rounded_rectangle(
        (folder_left + stroke, body_top + stroke,
         folder_right - stroke, folder_bottom - stroke),
        radius=r_in, fill=255,
    )
    idr.rounded_rectangle(
        (folder_left + stroke, folder_top + stroke,
         tab_right - stroke, body_top + r_out + stroke),
        radius=r_in, fill=255,
    )
    return ImageChops.subtract(outer, inner)


def circle_ring_mask(s, cx, cy, radius, stroke, gap_angle_deg=28.0):
    outer = Image.new("L", (s, s), 0)
    ImageDraw.Draw(outer).ellipse(
        (cx - radius, cy - radius, cx + radius, cy + radius), fill=255,
    )
    inner = Image.new("L", (s, s), 0)
    inner_r = radius - stroke
    ImageDraw.Draw(inner).ellipse(
        (cx - inner_r, cy - inner_r, cx + inner_r, cy + inner_r), fill=255,
    )
    ring = ImageChops.subtract(outer, inner)

    gap = Image.new("L", (s, s), 0)
    gd = ImageDraw.Draw(gap)
    gd.pieslice(
        (cx - radius - stroke, cy - radius - stroke,
         cx + radius + stroke, cy + radius + stroke),
        start=-gap_angle_deg / 2, end=gap_angle_deg / 2,
        fill=255,
    )
    return ImageChops.subtract(ring, gap)


def waveform_mask(s, cx, cy):
    bar_w = s * 0.030
    bar_gap = s * 0.022
    bar_heights_frac = [0.07, 0.115, 0.165, 0.115, 0.07]
    bar_count = len(bar_heights_frac)

    mask = Image.new("L", (s, s), 0)
    d = ImageDraw.Draw(mask)
    total_w = bar_count * bar_w + (bar_count - 1) * bar_gap
    first_center_x = cx - total_w / 2 + bar_w / 2
    for i, hf in enumerate(bar_heights_frac):
        x = first_center_x + i * (bar_w + bar_gap)
        h = hf * s
        d.rounded_rectangle(
            (x - bar_w / 2, cy - h / 2, x + bar_w / 2, cy + h / 2),
            radius=bar_w / 2, fill=255,
        )
    return mask


# --- Compose -------------------------------------------------------------

def make_icon(size):
    s = size
    out = Image.new("RGBA", (s, s), (0, 0, 0, 0))

    bg = gradient_bg(s)
    rcorner = rounded_square_mask(s, corner_frac=0.22)
    out.paste(bg, (0, 0), rcorner)

    vignette = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    vd = ImageDraw.Draw(vignette)
    vd.ellipse(
        (s * 0.05, s * 0.10, s * 0.95, s * 1.05),
        fill=(76, 38, 140, 36),
    )
    vignette = vignette.filter(ImageFilter.GaussianBlur(radius=s * 0.08))
    va = vignette.split()[-1]
    vignette.putalpha(ImageChops.multiply(va, rcorner))
    out.alpha_composite(vignette)

    folder_stroke = max(int(s * 0.030), 2)
    folder_mask = folder_outline_mask(s, folder_stroke)

    circle_cx = s * 0.50
    circle_cy = s * 0.585
    circle_r = s * 0.175
    circle_stroke = max(int(s * 0.027), 2)
    circle_mask = circle_ring_mask(
        s, circle_cx, circle_cy, circle_r, circle_stroke, gap_angle_deg=24.0,
    )

    silver_mask = ImageChops.add(folder_mask, circle_mask)

    silver_halo = make_glow(silver_mask, PURPLE_GLOW,
                            blur=s * 0.026, opacity=0.55)
    out.alpha_composite(silver_halo)
    silver_halo_close = make_glow(silver_mask, PURPLE_GLOW,
                                  blur=s * 0.010, opacity=0.45)
    out.alpha_composite(silver_halo_close)

    silver_layer = fill_from_mask(silver_mask, SILVER)
    highlight = Image.new("L", (s, s), 0)
    hd = ImageDraw.Draw(highlight)
    for y in range(s):
        t = max(0.0, 1.0 - y / (s * 0.6))
        hd.line([(0, y), (s, y)], fill=int(80 * t))
    sheen_mask = ImageChops.multiply(silver_mask, highlight)
    sheen = fill_from_mask(sheen_mask, SILVER_HIGHLIGHT)
    silver_layer = Image.alpha_composite(silver_layer, sheen)
    out.alpha_composite(silver_layer)

    wave_mask = waveform_mask(s, circle_cx, circle_cy)

    wave_glow_far = make_glow(wave_mask, PURPLE_GLOW,
                              blur=s * 0.07, opacity=0.70)
    wave_glow_mid = make_glow(wave_mask, PURPLE_GLOW,
                              blur=s * 0.028, opacity=0.85)
    wave_glow_near = make_glow(wave_mask, PURPLE_BAR,
                               blur=s * 0.010, opacity=0.85)
    bars = fill_from_mask(wave_mask, PURPLE_BAR)
    inner_hot_mask = wave_mask.filter(ImageFilter.GaussianBlur(radius=s * 0.005))
    inner_hot_mask = ImageChops.subtract(
        wave_mask, inner_hot_mask.point(lambda v: max(0, v - 80))
    )
    bars_highlight = fill_from_mask(inner_hot_mask, PURPLE_BAR_HOT)

    wave_composite = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    wave_composite.alpha_composite(wave_glow_far)
    wave_composite.alpha_composite(wave_glow_mid)
    wave_composite.alpha_composite(wave_glow_near)
    wave_composite.alpha_composite(bars)
    wave_composite.alpha_composite(bars_highlight)

    wa = wave_composite.split()[-1]
    wave_composite.putalpha(ImageChops.multiply(wa, rcorner))
    out.alpha_composite(wave_composite)

    rim = Image.new("L", (s, s), 0)
    rd = ImageDraw.Draw(rim)
    rd.rounded_rectangle((0, 0, s, s),
                         radius=max(int(s * 0.22), 1), outline=255,
                         width=max(int(s * 0.012), 1))
    rim = rim.filter(ImageFilter.GaussianBlur(radius=s * 0.010))
    rim_layer = fill_from_mask(rim, (255, 255, 255))
    ra = rim_layer.split()[-1].point(lambda v: int(v * 0.06))
    rim_layer.putalpha(ImageChops.multiply(ra, rcorner))
    out.alpha_composite(rim_layer)

    return out


# --- File output ---------------------------------------------------------

def save_pngs():
    icons_at = {}
    for name, size in SIZES_PNG.items():
        if size not in icons_at:
            icons_at[size] = make_icon(size)
        icons_at[size].save(HERE / name, "PNG")
        print(f"wrote {name} ({size}x{size})")
    return icons_at


def save_ico(icons):
    needed = [16, 32, 48, 64, 128, 256]
    rendered = {sz: icons.get(sz) or make_icon(sz) for sz in needed}
    rendered[needed[0]].save(
        HERE / "icon.ico",
        format="ICO",
        sizes=[(sz, sz) for sz in needed],
        append_images=[rendered[sz] for sz in needed[1:]],
    )
    print("wrote icon.ico (multi-res)")


def save_icns(icons):
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


def _to_png_bytes(img):
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
