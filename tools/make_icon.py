#!/usr/bin/env python3
"""Generate the app icon (white rounded square with the Qudelix 5K's top end (jacks)) for both widgets.

* windows/icon.ico                       — PNG-in-ICO, 16…256 px (embedded by windows/build.rs)
* android/res/drawable/ic_widget.xml     — the same geometry as an Android vector

Dependency-free: rasterises with 4x4 supersampling and writes PNG via zlib.
Run:  python3 tools/make_icon.py
"""
import math
import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

RED = (0xE5, 0x39, 0x35)
WHITE = (0xFF, 0xFF, 0xFF)

# Shapes in a 48x48 viewport, painted in order (later wins): a white rounded square with the
# Qudelix 5K's top end (the two jacks, keys as ears at its sides) taken from the
# vector model in tools/icon_parts.py (true proportions), centred.
import icon_parts

TOP_FX = ("springs2",)          # applied model fixes for the top face


def top_shapes(bg, width=38.0, cx=24.0, cy=24.0):
    x0, x1, y0, y1 = -1.24, 30.4 + 1.06, 0.0, 17.98                 # face + key ears, clip included
    k = width / (x1 - x0)
    ox, oy = cx - (x0 + x1) / 2 * k, cy - (y0 + y1) / 2 * k
    f = lambda x, y: (round(ox + x * k, 3), round(oy + y * k, 3))
    out = []
    for kind, *a in icon_parts.top_new(fx=TOP_FX):
        c = bg if a[-1] == icon_parts.SEE_THROUGH else a[-1]        # see-through gaps show the background
        if kind == "rrect":
            out.append(("rrect", *f(a[0], a[1]), *f(a[2], a[3]), round(a[4] * k, 3), c))
        elif kind == "circle":
            out.append(("circle", *f(a[0], a[1]), round(a[2] * k, 3), c))
        elif kind == "line":
            out.append(("line", *f(a[0], a[1]), *f(a[2], a[3]), round(a[4] * k, 3), c))
        else:
            out.append(("poly", [f(x, y) for x, y in a[0]], c))
    return out


SQUARE = ("rrect", 2, 2, 46, 46, 9)                              # rounded square behind the device
SHAPES = [SQUARE + (WHITE,)] + top_shapes(WHITE)                  # default: white

# Background presets offered in both apps' settings (Android can only switch
# between prebuilt icons; Windows also takes any colour — see windows/src/logo.rs).
PRESETS = [("white", WHITE), ("red", RED), ("black", (0x12, 0x12, 0x14)), ("orange", (0xF9, 0x73, 0x16)),
           ("amber", (0xF5, 0x9E, 0x0B)), ("green", (0x10, 0xB9, 0x81)), ("blue", (0x2F, 0x6F, 0xEB)),
           ("purple", (0x8B, 0x5C, 0xF6))]
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 256)


def in_poly(x, y, pts):
    inside = False
    j = len(pts) - 1
    for i in range(len(pts)):
        (xi, yi), (xj, yj) = pts[i], pts[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
            inside = not inside
        j = i
    return inside


def in_rrect(x, y, x0, y0, x1, y1, r):
    if not (x0 <= x <= x1 and y0 <= y <= y1):
        return False
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def hit(shape, x, y):
    kind, *a = shape
    if kind == "rrect":
        return in_rrect(x, y, *a[:5])
    if kind == "circle":
        return math.hypot(x - a[0], y - a[1]) <= a[2]
    if kind == "line":
        return seg_dist(x, y, *a[:4]) <= a[4] / 2
    if kind == "poly":
        return in_poly(x, y, a[0])
    if kind == "ellipse":
        return ((x - a[0]) / a[2]) ** 2 + ((y - a[1]) / a[3]) ** 2 <= 1
    raise ValueError(kind)


def sample(x, y):
    """Colour at viewport point (x, y), or None outside the icon."""
    color = None
    for sh in SHAPES:
        if hit(sh, x, y):
            color = sh[-1]
    return color


def write_layers(path, sizes=ICO_SIZES, ss=4):
    """Windows: the icon split into the rounded square's coverage (1 byte/px) and the
    device on a transparent background (RGBA), per size, so the app can put any
    background colour behind the device at run time. See windows/src/logo.rs."""
    global SHAPES
    keep, cut = SHAPES, (1, 2, 3)                                  # a colour no shape uses = "see-through"
    out = bytearray()
    for size in sizes:
        SHAPES = [SQUARE + (WHITE,)]
        out += bytes(r[i] for r in render(size, ss) for i in range(3, len(r), 4))
        SHAPES = top_shapes(cut)
        for r in render(size, ss, cut=cut):
            out += r
    SHAPES = keep
    with open(path, "wb") as f:
        f.write(bytes(out))


def render(size, ss=4, cut=None):
    rows = []
    for py in range(size):
        row = bytearray()
        for px in range(size):
            acc = [0, 0, 0]
            hits = 0
            for sy in range(ss):
                for sx in range(ss):
                    c = sample((px + (sx + 0.5) / ss) * 48 / size, (py + (sy + 0.5) / ss) * 48 / size)
                    if c and c != cut:
                        hits += 1
                        for i in range(3):
                            acc[i] += c[i]
            if hits:
                row += bytes([acc[0] // hits, acc[1] // hits, acc[2] // hits, 255 * hits // (ss * ss)])
            else:
                row += b"\0\0\0\0"
        rows.append(bytes(row))
    return rows


def png(size):
    raw = b"".join(b"\0" + r for r in render(size))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def write_ico(path, sizes=(16, 20, 24, 32, 40, 48, 64, 256)):
    images = [png(s) for s in sizes]
    head = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries = b""
    for s, img in zip(sizes, images):
        entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(img), offset)
        offset += len(img)
    with open(path, "wb") as f:
        f.write(head + entries + b"".join(images))


def write_vector(path):
    hexc = lambda c: "#%02X%02X%02X" % c
    paths = []
    for kind, *a in SHAPES:
        col = hexc(a[-1])
        if kind == "rrect":
            x0, y0, x1, y1, r = a[:5]
            d = (f"M{x0 + r},{y0} H{x1 - r} A{r},{r} 0 0 1 {x1},{y0 + r} V{y1 - r} "
                 f"A{r},{r} 0 0 1 {x1 - r},{y1} H{x0 + r} A{r},{r} 0 0 1 {x0},{y1 - r} V{y0 + r} "
                 f"A{r},{r} 0 0 1 {x0 + r},{y0} Z")
            paths.append(f'    <path android:fillColor="{col}" android:pathData="{d}" />')
        elif kind == "circle":
            cx, cy, r = a[:3]
            d = f"M{cx - r},{cy} A{r},{r} 0 1 0 {cx + r},{cy} A{r},{r} 0 1 0 {cx - r},{cy} Z"
            paths.append(f'    <path android:fillColor="{col}" android:pathData="{d}" />')
        elif kind == "poly":
            d = "M" + " L".join(f"{x},{y}" for x, y in a[0]) + " Z"
            paths.append(f'    <path android:fillColor="{col}" android:pathData="{d}" />')
        elif kind == "ellipse":
            cx, cy, rx, ry = a[:4]
            d = f"M{cx - rx},{cy} A{rx},{ry} 0 1 0 {cx + rx},{cy} A{rx},{ry} 0 1 0 {cx - rx},{cy} Z"
            paths.append(f'    <path android:fillColor="{col}" android:pathData="{d}" />')
        elif kind == "line":
            ax, ay, bx, by, w = a[:5]
            paths.append(f'    <path android:strokeColor="{col}" android:strokeWidth="{w}" '
                         f'android:strokeLineCap="round" android:pathData="M{ax},{ay} L{bx},{by}" />')
    xml = ("""<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by tools/make_icon.py — white rounded square with the Qudelix 5K top end (same as windows/icon.ico). -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="48dp" android:height="48dp"
    android:viewportWidth="48" android:viewportHeight="48">
""" + "\n".join(paths) + "\n</vector>\n")
    with open(path, "w") as f:
        f.write(xml)


if __name__ == "__main__":
    write_ico(os.path.join(ROOT, "windows", "icon.ico"))
    with open(os.path.join(ROOT, "windows", "icon-preview.png"), "wb") as f:
        f.write(png(128))
    write_layers(os.path.join(ROOT, "windows", "logo-layers.bin"))
    drawable = os.path.join(ROOT, "android", "res", "drawable")
    write_vector(os.path.join(drawable, "ic_widget.xml"))
    for name, bg in PRESETS:                                      # launcher-icon choices (activity-aliases)
        SHAPES = [SQUARE + (bg,)] + top_shapes(bg)
        write_vector(os.path.join(drawable, f"ic_logo_{name}.xml"))
    print("icon.ico, icon-preview.png, logo-layers.bin, ic_widget.xml, ic_logo_*.xml written")
