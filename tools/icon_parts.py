#!/usr/bin/env python3
"""Draw each side of the Qudelix 5K separately and large (a review sheet), so
every face can be checked against photos before it goes into the icon.

Units are millimetres (body ≈ 32 wide × 60 long × 17 thick). Each face is a
list of shapes in its own coordinates; `sheet()` lays them out and rasterises
with per-shape bounding boxes + 3x3 supersampling (dependency-free).

Run:  python3 tools/icon_parts.py <out.png>
"""
import math
import struct
import sys
import zlib

BODY = (0x47, 0x47, 0x4A)      # anodised black
BODY_HI = (0x5A, 0x5A, 0x5F)   # lit edge / seam
PLATE = (0x30, 0x30, 0x34)     # back steel plate
CLIP = (0x42, 0x42, 0x44)      # belt clip (darker, matte)
HOLE = (0x35, 0x35, 0x36)
SHADOW = (0x3A, 0x3A, 0x3B)    # shade between the body and the clip plate
RING = (0x4A, 0x4A, 0x50)
TEXT = (0x8E, 0x8E, 0x95)
STEEL = (0x6E, 0x6E, 0x76)
GLINT = (0xB4, 0xB4, 0xBC)    # spring highlight
KEY = (0x9C, 0x9C, 0xA4)       # translucent key (unlit)
RED, RED_HI = (0xE5, 0x39, 0x35), (0xFF, 0x9A, 0x90)
BLUE, BLUE_HI = (0x2F, 0x6F, 0xEB), (0x9F, 0xC4, 0xFF)


def rr(x0, y0, x1, y1, r, c):
    return ("rrect", x0, y0, x1, y1, r, c)


def circle(cx, cy, r, c):
    return ("circle", cx, cy, r, c)


def line(ax, ay, bx, by, w, c):
    return ("line", ax, ay, bx, by, w, c)


# ── 1: top end (32 × 17), looking at the jacks; back plate side at the top ──
def top_end():
    return [
        rr(-2.2, 6.5, 2.0, 12.0, 0.8, RED), rr(-1.6, 7.3, 0.0, 11.2, 0.4, RED_HI),        # key ear (left)
        rr(30.0, 6.5, 34.2, 12.0, 0.8, BLUE), rr(32.0, 7.3, 33.6, 11.2, 0.4, BLUE_HI),    # key ear (right)
        rr(0, 0, 32, 17, 1.4, BODY),
        line(0.6, 16.4, 31.4, 16.4, 0.5, BODY_HI),                                        # seam to the front
        circle(16, 5.6, 2.5, RING), circle(16, 5.6, 1.6, HOLE),                            # 2.5 mm (nearer the back)
        circle(16, 11.4, 3.1, RING), circle(16, 11.4, 2.1, HOLE),                          # 3.5 mm
        circle(20.6, 6.6, 0.35, HOLE),                                                    # mic hole
    ]


# ── 2: back (32 × 60): steel plate, slots, clip, label, keys peeking ──
def back():
    s = [
        rr(-0.7, 3.0, 0.4, 16.0, 0.4, RED), rr(31.6, 3.0, 32.7, 16.0, 0.4, BLUE),         # keys peek out
        rr(0, 0, 32, 60, 1.6, PLATE),
        rr(4.2, 2.2, 10.2, 3.9, 0.85, HOLE), rr(21.8, 2.2, 27.8, 3.9, 0.85, HOLE),        # slots
        rr(10.8, 0, 21.2, 24.0, 1.0, CLIP),                                               # belt clip
        line(11.3, 0.4, 20.7, 0.4, 0.5, BODY_HI),
    ]
    rows = [(10, 44.0), (7, 45.6), (8, 47.2), (6.5, 48.8), (8, 50.4), (5, 52.0)]
    for w, y in rows:                                                                     # label text
        s.append(line(16 - w / 2, y, 16 + w / 2, y, 0.55, TEXT))
    s += [circle(12.8, 54.2, 0.9, TEXT), circle(12.8, 54.2, 0.55, PLATE),
          rr(15.0, 53.4, 16.6, 55.0, 0.2, TEXT), line(17.8, 54.2, 21.5, 54.2, 0.55, TEXT)]
    return s


# ── 3: bottom end (32 × 17): the end cap (U-shaped assembly seam), USB-C in the
#    middle between two torx screws at the same height; below it, past a gap, the
#    edge of the back plate (as in the photo of the bottom).
def torx(cx, cy):
    s = [circle(cx, cy, 1.75, RING), circle(cx, cy, 1.25, BODY)]
    for k in range(3):                                              # 6-point star
        a = math.radians(k * 60)
        s.append(line(cx - 0.75 * math.cos(a), cy - 0.75 * math.sin(a),
                      cx + 0.75 * math.cos(a), cy + 0.75 * math.sin(a), 0.38, RING))
    s.append(circle(cx, cy, 0.25, BODY))
    return s


def bottom_end():
    return [
        rr(0, 0, 32, 13.4, 1.2, BODY),                                                  # end cap
        # assembly seam: U-shaped — down the left, along the bottom, up the right (none on top)
        line(1.0, 1.2, 1.0, 12.3, 0.45, BODY_HI),
        line(1.0, 12.3, 31.0, 12.3, 0.45, BODY_HI),
        line(31.0, 12.3, 31.0, 1.2, 0.45, BODY_HI),
        rr(11.3, 5.9, 20.7, 9.3, 1.7, RING), rr(11.8, 6.35, 20.2, 8.85, 1.25, HOLE),   # USB-C
        line(13.4, 7.6, 18.6, 7.6, 0.6, BODY_HI),                                       # contact tongue
        *torx(5.2, 7.6), *torx(26.8, 7.6),
        # between the body and the clip plate: not empty — a shadow, with the clip's
        # two springs visible and its hook in the middle
        rr(0.4, 13.0, 31.6, 14.8, 0.3, SHADOW),
        *spring(6.4), *spring(24.0),
        rr(0, 14.6, 32, 16.6, 0.7, PLATE),                                              # clip plate: same width as the body
        # hook: the clip plate's tip bent IN toward the body across the gap (like a
        # clothes-peg tip), in the middle
        # same width as the clip on the back and the SAME colour as the plate — only
        # the shading differs: the body's shadow falls across its top edge
        rr(10.8, 12.95, 21.2, 15.6, 0.6, PLATE),
        line(11.2, 13.15, 20.8, 13.15, 0.45, SHADOW),
        line(10.95, 13.3, 10.95, 14.6, 0.3, SHADOW), line(21.05, 13.3, 21.05, 14.6, 0.3, SHADOW),
    ]



def bottom_new(W=30.4, r=0.25, fx=()):
    """USB-C end — v3 from 4 near head-on photos (rectified to model mm), drawing for exact proportions."""
    o = 0.09

    def R(x0, y0, x1, y1, rad, c):
        return [rr(x0 - o, y0 - o, x1 + o, y1 + o, rad + o, OUTLINE), rr(x0, y0, x1, y1, rad, c)]

    def screw(cx, cy):                                   # counterbored socket screw: dark recess, lit rim, metal head, hex socket
        hexagon = [(cx + 0.36 * math.cos(math.radians(30 + 60 * k)), cy + 0.36 * math.sin(math.radians(30 + 60 * k))) for k in range(6)]
        return [circle(cx, cy, 1.85, BODY_HI), circle(cx, cy, 1.75, HOLE),       # rim highlight + counterbore
                circle(cx, cy, 1.3, RING), circle(cx, cy, 1.05, STEEL),          # head: edge + face
                ("poly", hexagon, HOLE)]                                          # hex socket

    def groove(x0, y0, x1, y1):                          # panel line: dark groove + thin lit edge below/right of it
        return [line(x0, y0, x1, y1, 0.16, HOLE), line(x0 + 0.1 * (y1 != y0), y0 + 0.1 * (y1 == y0), x1 + 0.1 * (y1 != y0), y1 + 0.1 * (y1 == y0), 0.06, BODY_HI)]

    cx0 = W / 2
    s = []
    s += R(-1.24, 8.3, 0.3, 12.1, r, BLUE)               # Power key sticks out on the left (drawing)
    s += R(W - 0.3, 8.0, W + 1.06, 11.9, r, RED)         # Volume key sticks out on the right
    s += R(0, 3.36, W, 18.31, r, BODY)                   # end face
    # the gap under the clip is SEE-THROUGH at both sides (photos), with a dark block in the middle
    s += [rr(0.2, 1.43, W - 0.15, 3.36, 0, SEE_THROUGH)]
    s += R(2.0, 1.43, 22.3, 3.36, 0.1, BODY)             # dark block seen through the gap (head-on photo 11c: 2.0 … 22.3)
    s += [rr(2.0, 1.43, 8.0, 3.36, 0.1, SHADOW)]         # its left part is deeper / darker (drawing divider at 8.1)
    s += R(21.6, 1.6, 23.3, 3.36, 0.5, BODY_HI)          # small rounded lug at its right end
    if 'springs2' in fx:                                 # thick short turns standing on the body, mirrored (f008428b / a4c95a21)
        s += [rr(23.3, 1.43, 26.7, 3.36, 0.1, SHADOW)] + turns(5.1, 7.8, 4, 2.2, 3.15) + turns(23.6, 26.3, 4, 2.2, 3.15)   # right one just past the lug
    elif 'coil' in fx:                                   # clip springs show in the gap at BOTH sides (tilted photos a4c / f00)
        # box drawing: "=" spring at both sides, mirrored (3.1–4.4 / 26.0–27.3), each in a dark pocket
        s += [rr(W - 8.0, 1.43, W - 2.0, 3.36, 0.1, SHADOW)] + coil(2.9, 4.7, 1.7, 3.15) + coil(W - 4.7, W - 2.9, 1.7, 3.15)
    else:
        s += [line(3.0 + 0.5 * k, 2.2, 3.0 + 0.5 * k, 3.0, 0.18, STEEL) for k in range(4)]   # spring contacts "||||" (f00; drawing "=" at 2.8–4.75)
    # right of the lug the gap is open (both head-on photos); the drawing's right "=" is not visible head-on
    # inset panel: groove across at 4.9, down both sides 1.15 in from the edges, ends at 17.0 with a tick out to the edge
    a, b, yt, yb, q = 1.15, W - 1.15, 4.9, 17.0, 0.25
    s += groove(a + q, yt, b - q, yt) + groove(a, yt + q, a, yb) + groove(b, yt + q, b, yb)
    for ccx, sx in ((a + q, -1), (b - q, 1)):            # r 0.25 corners
        pts = [(ccx + sx * q * math.sin(t), yt + q - q * math.cos(t)) for t in [k * math.pi / 16 for k in range(9)]]
        s += [line(*pts[i], *pts[i + 1], 0.16, HOLE) for i in range(8)]
    s += [line(0, yb, a, yb, 0.16, HOLE), line(b, yb, W, yb, 0.16, HOLE)]
    # USB-C, high (level with the screws): recess with a lit rim, dark opening, tongue in its upper half
    cx, cy = cx0 - 0.05, 7.85
    s += [rr(cx - 5.05, cy - 1.75, cx + 5.05, cy + 1.75, 1.75, BODY_HI), rr(cx - 4.95, cy - 1.65, cx + 4.95, cy + 1.65, 1.65, OUTLINE),
          rr(cx - 4.45, cy - 1.25, cx + 4.45, cy + 1.25, 1.25, HOLE),
          rr(cx - 3.65, cy - 0.5, cx + 3.65, cy - 0.02, 0.24, RING)]
    s += screw(4.7, 7.75) + screw(W - 4.7, 7.75)
    s += R(0.4, 0.33, W - 0.25, 1.43, r, CLIP_SCAN)      # clip (photos: ≈0.4 … W-0.25)
    return s


def back_new(W=30.4, L=60.0, r=0.25, rb=0.5):
    """Back = the outside of the clip plate. Jack end UP (y=0, meets top_new's clip edge), USB-C end DOWN
    (y=L, meets bottom_new's clip edge); seen from outside: Volume (red) on the RIGHT, Power on the left.
    y_b = 60 - x_side.  rb: the clip plate's own corner radius (the one allowed 'other' radius)."""
    o = 0.09
    def R(x0, y0, x1, y1, rad, c):
        return [rr(x0 - o, y0 - o, x1 + o, y1 + o, rad + o, OUTLINE), rr(x0, y0, x1, y1, rad, c)]
    tx0, tx1 = 10.3, 19.6                                # jack-end tongue (top_new), through the clip's slot
    s = []
    s += R(W - 0.3, -0.25, W + 1.06, 17.6, r, RED)       # Volume key edge, sticks out 1.06 (side: key x 42.4…60.25)
    s += R(-1.24, -0.25, 0.3, 17.6, r, BLUE)             # Power key edge, sticks out 1.24
    s += R(0, 0, W, L, r, BODY)                          # body: only a thin rim shows round the plate
    # the clip plate: 0.4 … W-0.25 across (bottom_new), from the jack end (flush) to 0.4 short of the USB-C end
    s += R(0.4, 0.0, W - 0.25, L - 0.4, rb, CLIP_SCAN)
    # slot in the clip at the jack end with the body tongue (the ear) filling it, flush with the plate
    s += R(tx0, -0.01, tx1, 60 - 43.35, r, BODY)
    return s


def turned(shapes, f):
    """Shapes moved by a 90° turn f(x, y) -> (x', y') (for laying the sides out round the back)."""
    out = []
    for kind, *a in shapes:
        if kind == "rrect":
            (ax, ay), (bx, by) = f(a[0], a[1]), f(a[2], a[3])
            out.append(("rrect", min(ax, bx), min(ay, by), max(ax, bx), max(ay, by), *a[4:]))
        elif kind == "circle":
            out.append(("circle", *f(a[0], a[1]), *a[2:]))
        elif kind == "line":
            out.append(("line", *f(a[0], a[1]), *f(a[2], a[3]), *a[4:]))
        else:
            out.append(("poly", [f(x, y) for x, y in a[0]], *a[1:]))
    return out


BRASS = (0xB8, 0x98, 0x5A)        # jack contact ring (photo 03575498: brass/gold)


def top_new(W=30.4, r=0.25, fx=()):
    """Jack end (final audit fixes gated by fx: jacks2, bridge, mic2, earnotch, sidewall, seam2, strut2, clip2)."""
    o = 0.09
    def R(x0, y0, x1, y1, rad, c):
        return [rr(x0 - o, y0 - o, x1 + o, y1 + o, rad + o, OUTLINE), rr(x0, y0, x1, y1, rad, c)]
    yb, yc, yo = 13.81, 16.88, 17.98
    tx0, tx1 = (10.0, 19.4) if 'clip2' in fx else (10.3, 19.6)      # tongue edges (photos 035/414)
    s = []
    s += R(0.2, yb - 0.2, tx0, yc, 0, SHADOW) + R(tx1, yb - 0.2, W - 0.2, yc, 0, SHADOW)
    s += turns(4.8, 7.6, 4, 14.0, 15.3) + turns(W - 7.6, W - 4.8, 4, 14.0, 15.3)   # identical springs, mirrored
    st = 0.65                                                         # strut on the niche floor (photo: ~0.65)
    se = 1.3                                                          # user: a bit shorter at the outer sides
    s += R(se, yc - st, tx0, yc, 0.1, STRUT) + R(tx1, yc - st, W - se, yc, 0.1, STRUT)
    fO, fF = R(0, 0, W, yb, r, BODY)
    tO, tF = R(tx0, yb - 1.0, tx1, yo, r, BODY)
    s += [tO, fO, tF, fF]
    sy = 1.25                                                         # seam near the front edge (photo 1.1, drawing 1.25)
    if True:                                                           # a seam is a groove sunk INTO the body: thin dark slit, lit lip below, ends flush with the sides
        s += [rr(0, sy - 0.08, W, sy + 0.08, 0, HOLE), rr(0, sy + 0.08, W, sy + 0.15, 0, BODY_HI)]
        h, d, ny = 0.17, 0.07, sy + 0.035                             # user: the groove runs out through the side — a shallow round notch in the edge, no bridge
        c = (h * h + d * d) / (2 * d)                                 # arc through the groove's span, depth d
        for xe, sx in ((0.0, -1), (W, 1)):
            s += [circle(xe + sx * (c - d), ny, c + o, OUTLINE), circle(xe + sx * (c - d), ny, c, SEE_THROUGH)]
            lo_, hi_ = sorted((xe + sx * o, xe + sx * (2 * c + o + 0.2)))
            s += [rr(lo_, ny - c - o - 0.05, hi_, ny + c + o + 0.05, 0, SEE_THROUGH)]
    else:
        s += [line(0, sy, W, sy, 0.3, JACK)]
    if 'sidewall' in fx:                                              # side-wall edge lines ~0.9 in from both edges (drawing double lines + lit band on photos), broken by the key ears
        for xw in (0.9, W - 0.9):
            s += [line(xw, sy + 0.4, xw, 5.6, 0.12, BODY_HI), line(xw, 10.8, xw, yb - 0.5, 0.12, BODY_HI)]
    jx = W / 2
    if True:                                                          # user: old size; big jack = black bezel, brass ring, dark centre — nothing else; no nub between the jacks
        jy1, jy2 = 6.7, 13.1
        s += [circle(jx, jy1, 3.5, HOLE), circle(jx, jy1, 3.38, BODY_HI), circle(jx, jy1, 2.35, HOLE),
              circle(jx, jy1, 1.4, BRASS), circle(jx, jy1, 1.15, HOLE)]
        s += [circle(jx, jy2, 2.7, HOLE), circle(jx, jy2, 2.58, BODY_HI), circle(jx, jy2, 1.7, HOLE),
              circle(jx - 0.3, jy2 + 1.2, 0.18, STEEL)]
    elif 'jacks2' in fx:                                                # photos: wider raised bezels, bigger openings, brass ring in the big jack
        jy1, jy2 = 6.3, 12.95
        s += [circle(jx, jy1, 3.95, HOLE), circle(jx, jy1, 3.8, RING), circle(jx, jy1, 2.75, HOLE),
              circle(jx, jy1, 1.6, BRASS), circle(jx, jy1, 1.3, HOLE)]
        s += [circle(jx, jy2, 3.0, HOLE), circle(jx, jy2, 2.85, RING), circle(jx, jy2, 1.7, HOLE),
              circle(jx - 0.3, jy2 + 1.2, 0.18, STEEL)]
        if 'bridge' in fx:                                            # the bezels touch; a small nub joins them (photo 035 + the drawing's "8")
            s += [rr(jx - 0.45, jy1 + 3.6, jx + 0.45, jy2 - 2.65, 0.2, RING)]
    else:
        s += [circle(jx, 6.7, 3.5, HOLE), circle(jx, 6.7, 3.32, RING), circle(jx, 6.7, 2.3, HOLE),
              circle(jx, 6.7, 1.45, STEEL), circle(jx, 6.7, 1.25, HOLE)]
        s += [circle(jx, 13.1, 2.7, HOLE), circle(jx, 13.1, 2.55, RING), circle(jx, 13.1, 1.7, HOLE),
              circle(jx - 0.3, 14.3, 0.18, STEEL)]
    if 'mic2' in fx:                                                  # photos: (7.25…7.9, 11.4…12.0), in a shallow dimple
        s += [circle(7.6, 11.75, 0.55, BODY_HI), circle(7.6, 11.75, 0.45, BODY), circle(7.6, 11.75, 0.3, HOLE)]
    else:
        s += [circle(8.6, 12.1, 0.32, HOLE)]
    for x0, x1, xe, c, ch, y0, y1, nx in ((-1.3, 2.2, 0.0, BLUE, BLUE_HI, 6.0, 10.0, 2.8),
                                          (W - 2.5, W + 1.1, W, RED, RED_HI, 6.05, 10.4, W - 3.8)):
        if 'earnotch' in fx:                                          # the key sits in a notch cut in the face; dark gap on its inner side
            nx0, nx1 = sorted((nx, xe))
            s += [rr(nx0, y0 - 0.05, nx1, y1 + 0.05, r, HOLE)]
            s += R(x0, y0, x1, y1, r, c)
        else:
            x0, x1, y0, y1 = ((-1.24, 3.0, 6.21, 10.01) if xe == 0.0 else (W - 3.0, W + 1.06, 6.41, 10.31))
            s += R(x0, y0, x1, y1, r, c)
        s += [line(xe, y0 + 0.15, xe, y1 - 0.15, 0.14, OUTLINE)]
        hx = x1 - 0.3 if xe == 0.0 else x0 + 0.3
        s += [line(hx, y0 + 0.3, hx, y1 - 0.3, 0.18, ch)]
    cr = W - 0.7 if 'clip2' in fx else W - 0.2                        # clip right end (both photos ≈29.7)
    s += R(0.2, yc, tx0, yo, r, CLIP_SCAN) + R(tx1, yc, cr, yo, r, CLIP_SCAN)
    return s


def coil(x0, x1, y0, y1, pitch=0.5):
    """Coil spring lying along x, seen side-on: near-upright wire turns, each lit at its top and bottom."""
    s, n = [], int((x1 - x0) / pitch)
    for k in range(n):
        x = x0 + k * pitch
        s += [line(x, y0, x + 0.12, y1, 0.16, RING),                     # wire turn, nearly upright (photo)
              line(x, y0, x + 0.03, y0 + 0.35, 0.12, GLINT), line(x + 0.09, y1 - 0.35, x + 0.12, y1, 0.12, GLINT)]
    return s


def turns(x0, x1, n, y0, y1, glint_top=True):
    """Clip spring lying along x, seen side-on: n thick wire turns, lit only at one end (photos)."""
    s, pitch = [], (x1 - x0 - 0.32) / max(n - 1, 1)
    for k in range(n):
        x = x0 + k * pitch
        s += [rr(x, y0, x + 0.32, y1, 0.16, RING)]                                    # turn, dim metal
        gy = y0 + 0.1 if glint_top else y1 - 0.45
        s += [rr(x + 0.04, gy, x + 0.28, gy + 0.35, 0.12, GLINT)]                    # glint
    return s


def spring(x0):
    """A small coil spring seen from the end, in the shadow gap."""
    return [line(x0 + k * 0.55, 13.2, x0 + k * 0.55 + 0.4, 14.6, 0.28, STEEL) for k in range(5)]


# ── 4/5: long sides (60 × 18.6), traced from the user's photos of the real side
#    (1 px ≈ 0.10 mm on the full-side photo; detail from the close-up):
#    thin CLIP along the top; under it an OPEN gap (background shows through) with
#    two spring posts near the far end; at the key end the body rises in a
#    "tower" with slanted shoulders up to the clip, a small pin hole in it; a grey
#    hook ledge under the clip just before the tower; a dark recess after it; the
#    end of the body is a solid column up to the clip; a vertical seam separates
#    the key-end piece; the long Volume key sits in a recessed slot in the upper
#    part, reaching the end and poking out a little; a seam runs near the bottom.
SLOT = (0x3C, 0x3C, 0x3E)


def omega(cx, top, r, base, half, f=0.9, n=80):
    """Outline of a round boss rising from a flat edge at y=`base` (y grows
    downward): a circle of radius r with its top at y=`top`, joined to the edge by
    concave fillets of radius f tangent to both ("Ω"). Returns polygon points."""
    cy = top + r                          # boss centre
    fcy = base - f                        # fillet centres sit f above the edge
    dx = math.sqrt(max(0.0, (r + f) ** 2 - (cy - fcy) ** 2))
    xt = dx * r / (r + f)                 # where fillet meets the boss (offset from cx)

    def y_at(x):
        d = abs(x - cx)
        if d <= xt:                       # on the boss
            return cy - math.sqrt(max(0.0, r * r - d * d))
        if d <= dx:                       # on the concave fillet (lower arc)
            return fcy + math.sqrt(max(0.0, f * f - (d - dx) ** 2))
        return base                       # flat edge
    pts = [(cx - half + 2 * half * i / n, 0.0) for i in range(n + 1)]
    pts = [(x, y_at(x)) for x, _ in pts]
    return [(cx - half, base + 0.3)] + pts + [(cx + half, base + 0.3)]


# Tones (user, 2026-10-10): body black; clip a slightly dark grey; the clip's edge
# line DARKER than the clip (it's in shadow); the clip's tab darker grey still; the
# post hanging from the body blacker than the body itself.
CLIP_LIGHT = (0x48, 0x48, 0x4E)     # clip
CLIP_EDGE = (0x30, 0x30, 0x35)      # clip edge line, in shadow
CLIP_TAB = (0x34, 0x34, 0x39)       # the clip's tab near the far end
POST_BLACK = (0x34, 0x34, 0x35)     # post: blacker than the body


def side(key, key_hi, mirror=False, step=5):
    """`step` applies the user's review fixes cumulatively (for the step-by-step
    review images): 1 the ears' mount = TWO strips under the clip, one on each
    side of the ears, 2 symmetric ears, 3 lighter clip, 4 darker spring posts.
    (A "longer part after the ears" variant was rejected — the ears stay put.)"""
    m = (lambda x: 60 - x) if mirror else (lambda x: x)
    def mrr(x0, y0, x1, y1, r, c):
        a, b = sorted((m(x0), m(x1)))
        return rr(a, y0, b, y1, r, c)
    def mpoly(pts, c):
        return ("poly", [(m(x), y) for x, y in pts], c)
    # mount ("ears") position/size: step 3 moves it away from the end so the part
    # after it is as long as on the real device
    t0, t1 = 43.0, 49.9
    tc = (t0 + t1) / 2
    clip_c = PLATE if step < 3 else CLIP_LIGHT
    post1, post2 = STEEL, STEEL
    s = [
        mrr(0.0, 3.36, 60.0, 18.6, 0.7, BODY),                       # body
    ]
    if step < 4:
        s += [mrr(0.0, 3.36, 1.7, 3.9, 0.1, SHADOW),
              mrr(5.4, 1.0, 6.3, 3.4, 0.2, post1), mrr(9.9, 1.0, 10.9, 3.4, 0.2, post2)]
    else:
        # far end, from the photo with the clip opened: the body steps down toward
        # the clip at the very end; the clip's light-grey tab rises from the clip;
        # a black post hangs from the body a bit further in
        s += [mrr(0.0, 2.55, 5.9, 4.6, 0.4, BODY),                     # step: one piece with the body
              mrr(8.2, 1.0, 9.8, 2.85, 0.2, CLIP_TAB),
              mrr(13.4, 1.55, 14.4, 3.5, 0.2, POST_BLACK)]
    s += [
        mrr(t1, 1.0, 58.4, 3.4, 0.2, SHADOW),                         # dark piece after the ears
    ]
    if step < 2:
        s.append(mpoly([(43.0, 3.4), (44.0, 1.5), (45.0, 1.0), (48.2, 1.0), (49.2, 1.5), (49.9, 3.4)], BODY))
    else:
        # the ears = a round boss around the pin hole: the body's clip-side edge
        # bulges in a half-circle toward the clip with smooth concave fillets on
        # both sides ("Ω"), as in the close-up photo
        s.append(mpoly(omega(tc, top=1.25, r=1.8, base=3.36, half=3.8, f=0.9), BODY))
    hy = 3.15 if step >= 2 else 2.2
    s += [circle(m(tc if step >= 2 else 46.6), hy, 0.55, RING), circle(m(tc if step >= 2 else 46.6), hy, 0.28, HOLE)]
    if step < 1:
        s.append(mrr(40.4, 1.0, 44.2, 1.7, 0.3, PLATE))              # one strip only (before step 1)
    else:                                                             # ears' mount: two strips under the clip
        s += [mpoly([(t0 - 3.4, 1.0), (t0 + 0.8, 1.0), (t0 + 0.8, 1.75), (t0 - 2.8, 1.75)], PLATE),
              mpoly([(t1 - 0.8, 1.0), (t1 + 3.4, 1.0), (t1 + 2.8, 1.75), (t1 - 0.8, 1.75)], PLATE)]
    seam_x = 42.6
    s += [
        mrr(58.4, 0.9, 60.0, 18.6, 0.5, BODY),                        # end column up to the clip
        mrr(0.0, 0.0, 60.0, 1.0, 0.35, clip_c),                       # clip
        line(m(0.4), 0.22, m(59.6), 0.22, 0.28, CLIP_EDGE if step >= 3 else BODY_HI),  # clip edge, in shadow
        line(m(seam_x), 3.6, m(seam_x), 18.3, 0.3, BODY_HI),          # seam: key-end piece
        mrr(seam_x + 0.3, 7.7, 59.2, 11.3, 1.0, SLOT),                # recessed key slot
        mrr(seam_x + 0.7, 8.05, 60.5, 10.95, 0.9, key), mrr(seam_x + 1.4, 8.6, 59.9, 10.4, 0.6, key_hi),
        line(m(0.4), 16.6, m(59.6), 16.6, 0.35, BODY_HI),             # seam near the bottom
    ]
    return s


# ── 4/5 (current): sides TRACED from the coloured retail-box drawing ──
# (icon-modeling/trace.py → tools/side_volume_shapes.json: exact outlines in mm).
STRUT = (0x60, 0x60, 0x66)          # reinforcement struts by the ears: lighter than the clip
JACK = (0x35, 0x35, 0x36)           # block behind the ear (jack housing): blacker than the body
SEAM = (0x56, 0x56, 0x5A)


def side_traced(key, key_hi, mirror=False):
    import json, os
    d = json.load(open(os.path.join(os.path.dirname(__file__), "side_volume_shapes.json")))
    W = d["size_mm"][0]
    m = (lambda x: W - x) if mirror else (lambda x: x)
    col = {"body": BODY, "jack": JACK, "strut": STRUT, "clip": CLIP_LIGHT, "post_far": CLIP_TAB,
           "post_near": POST_BLACK, "hole": HOLE, "key": key, "seam": SEAM}
    out = []
    for sh in d["shapes"]:
        c = col[sh["part"]]
        if "poly" in sh:
            out.append(("poly", [(m(x), y) for x, y in sh["poly"]], c))
        else:
            x0, y0, x1, y1 = sh["rect"]
            a, b = sorted((m(x0), m(x1)))
            out.append(rr(a, y0, b, y1, 0.2, c))
    return out


# ── 4/5 (ideal): the clean vector model laid out EXACTLY on the coloured
# box-drawing scan (dimensions measured from the scan in mm, see
# icon-modeling/): straight edges, true circles, thin dark outlines like the scan.
OUTLINE = (0x3C, 0x3C, 0x3E)
CLIP_SCAN = (72, 72, 78)
STRUT_DARK = (52, 52, 57)          # clip struts: metal, darker than the clip (user)
STRUT_SCAN = (96, 96, 104)
SEE_THROUGH = (0xEC, 0xEC, 0xEE)   # see-through gap (background colour of the review sheets)
BLACKS = ("BODY", "BODY_HI", "CLIP", "HOLE", "SHADOW", "SLOT", "POST_BLACK", "JACK", "SEAM", "OUTLINE")


def lift_blacks(step):
    """Make every black a tone lighter (all by the same step, so their order is kept)."""
    g = globals()
    for n in BLACKS:
        g[n] = tuple(min(255, v + step) for v in g[n])


KEY_OUT = True    # the key sticks out 0.25 mm past the end (user, v45)


def side_ideal(key, key_hi, mirror=False, fx=(), split_key=True):   # split key on both sides (user, v51)
    # fx: scan-audit fixes to apply (shown to the user one by one as before/after)
    m = (lambda x: 60 - x) if mirror else (lambda x: x)
    o = 0.09                                            # thin outline, like the scan's fine lines

    def R(x0, y0, x1, y1, r, c, outline=True):
        a, b = sorted((m(x0), m(x1)))
        sh = [rr(a - o, y0 - o, b + o, y1 + o, r + o, OUTLINE)] if outline else []
        return sh + [rr(a, y0, b, y1, r, c)]

    def C(cx, cy, r, c, outline=True):
        sh = [circle(m(cx), cy, r + o, OUTLINE)] if outline else []
        return sh + [circle(m(cx), cy, r, c)]

    s = []
    # the side view uses ONE small corner radius almost everywhere (user)
    RS = 0.25                                                        # (applied v49)
    if True:   # bottom step (applied v50) — drawings + photo: from the far end to x≈8 the bottom edge sits 0.15 higher, then steps down
        bA = R(0.0, 4.5, 60.0, 18.16, RS, BODY)
        bB = R(8.0, 16.0, 60.0, 18.31, RS, BODY)
        s += [bB[0], bA[0], bB[1], bA[1]]                            # outlines first, then fills
    else:
        s += R(0.0, 4.5, 60.0, 18.31, RS or 0.8, BODY)               # body
    s += R(0.0, 3.36, 4.1, 5.0, 0.6, BODY)                           # far-end step toward the clip
    lo, hi = sorted((m(0.2), m(4.0)))
    s += [rr(lo, 4.4, hi, 5.2, 0.2, BODY)]                           # (merge the step into the body)
    lo, hi = sorted((m(3.4), m(4.5)))
    s += [rr(lo, 4.5, hi, 5.4, 0.0, BODY)]                           # hide the step's outline below the body's top edge (no intrusion)
    # straight far-end edge: the body's rounded top corner must not poke out under the step
    lo, hi = sorted((m(0.0), m(1.2)))
    s += [rr(lo, 4.0, hi, 6.0, 0.0, BODY)]
    edge_lo, edge_hi = sorted((m(-o), m(0.0)))
    s += [rr(edge_lo, 3.96, edge_hi, 17.5, 0.0, OUTLINE)]
    if True:   # step fillet (applied v43) — scan: concave fillet r≈0.35 where the step's wall meets the body top
        fr = RS or 0.35
        fcx, fcy = 4.1 + fr, 4.5 - fr
        def arc(r):
            return [(fcx - r * math.cos(t), fcy + r * math.sin(t)) for t in [k * math.pi / 2 / 32 for k in range(33)]]
        # outline ring (o wide, on the gap side), then the body fill up to the arc
        s += [("poly", [(m(x), y) for x, y in [(4.1 - 0.02, fcy - 0.02)] + arc(fr - o) + [(fcx + 0.02, 4.5 + 0.02), (4.1 - 0.02, 4.5 + 0.02)]], OUTLINE)]
        lo, hi = sorted((m(4.1 - 0.02), m(4.1 + o)))
        s += [rr(lo, fcy - 0.25, hi, fcy, 0.0, OUTLINE)]          # wall outline runs into the ring
        s += [("poly", [(m(x), y) for x, y in [(4.1 - 0.02, fcy - 0.02)] + arc(fr) + [(fcx + 0.02, 4.5 + 0.02), (4.1 - 0.02, 4.5 + 0.02)]], BODY)]
    if 'corner' in fx:   # scan: the far bottom corner is nearly square (r≈0.3), the key-end one stays 0.8
        lo, hi = sorted((m(-o), m(1.2)))
        s += [rr(lo, 17.0, hi, 18.31 + o, 0.3 + o, OUTLINE)]
        lo, hi = sorted((m(0.0), m(1.4)))
        s += [rr(lo, 16.95, hi, 18.31, 0.3, BODY)]
        lo, hi = sorted((m(0.0), m(1.6)))
        s += [rr(lo, 16.95, hi, 17.6, 0.0, BODY)]
    # seam: black like 5/19 (user), a little IN from the bottom edge as on the scan
    # (the scan's strip 3 — body colour — stays outside it), running the whole
    # length right up to both end edges. No band across the end face.
    s += [rr(min(m(0.0), m(60.0)), 16.45, max(m(0.0), m(60.0)), 16.95, 0.0, JACK)]
    # vertical seam right at the far-end edge, joining the bottom seam; a bit
    # short of the top so it doesn't run all the way up (user)
    lo, hi = sorted((m(0.0), m(0.5)))
    s += [rr(lo, 6.2, hi, 16.95, 0.0, JACK)]
    # at its top the seam tapers off into the side edge (user)
    s += [("poly", [(m(x), y) for x, y in [(0.0, 6.3), (0.5, 6.3), (0.5, 5.0), (0.48, 4.3),
                                             (0.42, 3.8), (0.3, 3.42),            # inner edge of the bend (v22 look — user)
                                             (0.15, 3.55), (0.05, 3.72), (0.0, 3.96)]], JACK)]   # curves out to the step's top corner
    # recess: a small, ROUNDED bite in the very edge right under the
    # rounded corner (where the user marked it) — shallow: ~0.055 mm deep, 0.32 mm tall.
    # Outline only inside the body (along the bite), outside stays background.
    # visible depth measured from the OUTER edge of the edge outline: v34 showed
    # o + 0.055 ≈ 0.145 mm; the user asked for half of that → 0.072 mm
    # nick shape drawn by the user: starts flush under the corner arc, curves in
    # (quarter ellipse) to full depth, straight inner wall, bottom shelf rounds out onto the edge
    yt, y1, y2, yb, dep = 3.90, 4.06, 4.14, 4.225, 0.085
    ell = lambda y: dep * math.sqrt(max(0.0, 1 - ((y1 - y) / (y1 - yt)) ** 2))
    # round the sharp tip where the cut meets the edge: a circle r=rf tangent to the
    # edge (d=0) and to the ellipse; find its centre height cy by bisection
    rf = 0.06
    eys = [yt + (y1 - yt) * k / 400 for k in range(401)]
    gap = lambda cy: min(math.hypot(rf - ell(y), cy - y) for y in eys) - rf
    lo_, hi_ = yt - rf, yt            # circle sits in the body corner above the tip: gap > 0 at lo, < 0 at hi
    for _ in range(50):
        mid = (lo_ + hi_) / 2
        lo_, hi_ = (mid, hi_) if gap(mid) > 0 else (lo_, mid)
    cy = lo_
    ty = min(eys, key=lambda y: math.hypot(rf - ell(y), cy - y))     # tangency on the ellipse
    def nick_d(y):
        if y < y1:   # top: concave quarter ellipse, its tip filleted
            if y < cy:
                return 0.0
            if y < ty:
                return rf - math.sqrt(max(0.0, rf * rf - (y - cy) ** 2))
            return ell(y)
        if y > y2:   # bottom: the body's corner is rounded (convex), shelf meets the edge tangentially
            return dep * (1 - math.sqrt(max(0.0, 1 - ((yb - y) / (yb - y2)) ** 2)))
        return dep
    prof = [(-o + nick_d(y), y) for y in [cy + (yb - cy) * k / 80 for k in range(81)]]
    # outline = the cut grown by o along its normal (discs along the profile), so the tips stay round
    s += [("circle", m(x), y, o, OUTLINE) for x, y in prof]
    # …but nothing may poke outside the corner arc above the cut (arc centre 0.6,3.96; outer r 0.6+o)
    ys = [yt - 0.2 + 0.2 * k / 20 for k in range(21)]
    s += [("poly", [(m(-0.4), ys[0])] + [(m(0.6 - math.sqrt((0.6 + o) ** 2 - (3.96 - y) ** 2)), y) for y in ys] + [(m(-0.4), ys[-1])], SEE_THROUGH)]
    s += [("poly", [(m(-0.4), cy)] + [(m(x), y) for x, y in prof] + [(m(-0.4), yb)], SEE_THROUGH)]
    yc, hh = (yt + yb) / 2, (yb - yt) / 2
    lo, hi = sorted((m(-0.4), m(-o)))
    s += [rr(lo, yc - hh - o - 0.01, hi, yc + hh + o + 0.01, 0.0, SEE_THROUGH)]   # nothing sticks out past the edge
    # spring posts in the see-through gap: far (scan 5) black, near (scan 7) clip colour — user
    s += R(6.7, 1.43, 8.0, 4.5, 0.15, JACK)
    s += R(11.0, 1.43, 12.4, 4.5, 0.15, CLIP_SCAN)                    # near post (scan 7): clip colour
    # key end (traced point by point from the scan, see compare-ear-*.png):
    # an ARCH-shaped ear grows straight out of the body — wide base, slanted
    # sides, rounded top just under the clip — with no see-through gaps beside
    # it: the black piece 10 on its left, the jack block (slanted left edge) on its
    # right; struts under the clip (the left one hooks down at its far end); the
    # pin hole is a dark disc with a black centre, no grey ring.
    def P(pts, c):
        return ("poly", [(m(x), y) for x, y in pts], c)
    sb = 4.5                                                         # shadow bottom: on the body's top line (scan)
    s += [P([(42.4, 1.43), (60.0, 1.43), (60.0, sb), (42.4, sb)], JACK)]   # shaded body under the clip, piece 10 → end (covers the body's top outline there)
    s += [P([(49.1, 2.1), (52.9, 2.1), (52.9, 1.43), (60.0, 1.43), (60.0, sb), (49.8, sb)], JACK)]   # shaded body right of the ear
    # 1 and 2 (piece 10 and the 'jack block') are the BODY itself, just in the
    # clip's shadow (user) — so no outline between them and the body, only darker
    lo, hi = sorted((m(42.4), m(44.2)))
    s += [rr(lo, 2.1, hi, sb, 0.3, JACK)]
    # …but the OUTER silhouette (shadowed body against the see-through gap and
    # past the end) keeps the thin outline: left + top edge of piece 1, the end edge
    lo, hi = sorted((m(42.4 - o), m(42.4)))
    s += [rr(lo, 2.1, hi, 4.5, 0.0, OUTLINE)]
    lo, hi = sorted((m(42.4 - o), m(44.2)))
    s += [rr(lo, 2.1 - o, hi, 2.1, 0.0, OUTLINE)]
    lo, hi = sorted((m(59.0), m(60.0)))
    s += [rr(lo, sb, hi, 5.6, 0.0, BODY)]                          # square body corner under the shadow: one straight end edge
    lo, hi = sorted((m(60.0), m(60.0 + o)))
    s += [rr(lo, 1.43, hi, 5.6, 0.0, OUTLINE)]
    if True:   # ear (applied v46) — scan: left side one straight slant, flat top against the clip, right side kinked at y≈2.5
        s += [P([(43.35, 4.7), (43.7, 3.9), (43.9, 3.36), (44.1, 3.0), (44.35, 2.5), (44.65, 2.1), (44.83, 1.65),
                 (45.2, 1.43), (47.6, 1.43), (47.95, 1.65), (48.4, 2.1), (48.9, 2.6), (49.05, 3.1), (49.25, 3.6),
                 (49.5, 4.2), (49.85, 4.7)], BODY)]
    else:
        s += [P([(44.5, 4.7), (44.5, 3.0), (44.8, 2.2), (45.5, 1.7), (46.6, 1.5), (47.6, 1.6),
                 (48.4, 1.95), (48.9, 2.6), (49.35, 3.6), (49.7, 4.7)], BODY)]                      # the ear (arch)
    if 'pin' in fx:   # photos: a metal pin — light ring, darker chamfer, dark hex centre
        s += [circle(m(46.5), 3.6, 0.8, (0x8A, 0x8A, 0x90)), circle(m(46.5), 3.6, 0.62, (0x5A, 0x5A, 0x60)),
              circle(m(46.5), 3.6, 0.35, HOLE)]
    else:
        s += C(46.55, 3.5, 1.15, OUTLINE, outline=False) + C(46.55, 3.5, 0.5, HOLE, outline=False)   # pin hole
    if True:   # struts (applied v48) — photos: the struts are metal, a bit DARKER than the clip (user), their inner ends wedge down along the ear's shoulders
        S9 = [(40.2, 1.43), (44.83, 1.43), (44.78, 1.71), (44.56, 2.15), (44.28, 2.43), (44.06, 2.37), (43.73, 2.1),
              (40.42, 2.1), (40.26, 1.99), (40.2, 1.82)]
        S18 = [(47.9, 1.43), (52.9, 1.43), (52.9, 1.75), (52.7, 2.0), (49.45, 2.05), (48.9, 2.48), (48.4, 2.05), (48.0, 1.65)]
        for pts in (S9, S18):
            s += [P(pts, STRUT_DARK)]
            for a_, b_ in zip(pts, pts[1:] + pts[:1]):          # thin outline, except along the clip
                if a_[1] > 1.44 or b_[1] > 1.44:
                    s += [("line", m(a_[0]), a_[1], m(b_[0]), b_[1], 2 * o, OUTLINE)]
    else:
        s += R(40.25, 1.43, 44.5, 1.96, 0.25, JACK)                  # left strut (scan 9): no hook, as on the scan
        s += R(48.3, 1.34, 52.9, 1.96, 0.25, JACK)                   # right strut (scan 18)
    if True:   # clip (applied v47) — photos + drawing: flush with the key end, only the top outer corner rounded (r 0.4),
        # square bottom corner sitting on the niche; far end 0.4 in from the body edge, small radii
        def rpoly(x0, y0, x1, y1, r):            # r = (top-left, top-right, bottom-right, bottom-left)
            pts = []
            for cx, cy, rad, a0 in [(x0 + r[0], y0 + r[0], r[0], 180), (x1 - r[1], y0 + r[1], r[1], 270),
                                    (x1 - r[2], y1 - r[2], r[2], 0), (x0 + r[3], y1 - r[3], r[3], 90)]:
                for k in range(13):
                    a = math.radians(a0 + 90 * k / 12)
                    pts.append((m(cx + rad * math.cos(a)), cy + rad * math.sin(a)))
            return pts
        cr = (RS,) * 4 if RS else (0.25, 0.4, 0.4, 0.2)   # key end rounded on BOTH corners (user)
        # under the clip's rounded bottom corner the dark niche/end wall shows, not background
        lo, hi = sorted((m(59.4), m(60.0)))
        s += [rr(lo, 0.95, hi, 1.6, 0.0, JACK)]
        lo, hi = sorted((m(60.0), m(60.0 + o)))
        s += [rr(lo, 0.95, hi, 1.6, 0.0, OUTLINE)]
        s += [("poly", rpoly(0.4 - o, 0.33 - o, 60.0 + o, 1.43 + o, tuple(v + o if v else 0 for v in cr)), OUTLINE),
              ("poly", rpoly(0.4, 0.33, 60.0, 1.43, cr), CLIP_SCAN)]
    else:
        s += R(0.5, 0.33, 59.7, 1.43, 0.45, CLIP_SCAN)               # clip: thin, as on the scan
    kx = 60.25 if 'keyout' in fx or KEY_OUT else 60.0                # the key sticks out a little past the end (user)
    s += R(42.4, 8.0, kx, 11.9, RS or 0.4, key)                            # Volume-Up key (corners r≈0.4 as on the scan)
    if split_key:   # other side (box drawing): the Power key has a cross groove near the end — two lines
        g0, g1 = 56.15, 57.63                                         # (pairing segment beyond it, up to the end)
        lo, hi = sorted((m(43.2), m(g0 - 0.7)))
        s += [rr(lo, 8.7, hi, 11.2, 0.15 if RS else 0.3, key_hi)]     # lit centre of the main part
        for gx in (g0, g1):
            s += [("line", m(gx), 8.0, m(gx), 11.9, 2 * o, OUTLINE)]
    else:
        lo, hi = sorted((m(43.2), m(kx - 0.8)))
        s += [rr(lo, 8.7, hi, 11.2, 0.15 if RS else 0.3, key_hi)]     # lit centre (user prefers this look)
    return s


FACES = [  # (shapes, width, height, (col, row) position in mm on the sheet)
    (top_end(), 32, 17, (6, 6)),
    (back(), 32, 60, (6, 30)),
    (bottom_end(), 32, 17, (6, 97)),
    (side_ideal(RED, RED_HI), 60, 17, (50, 30)),
    (side_ideal(BLUE, BLUE_HI, mirror=True, split_key=True), 60, 17, (50, 60)),
]
SHEET_W, SHEET_H, PX_PER_MM, SS = 118, 122, 8, 3
PAPER = (0xEC, 0xEC, 0xEE)


def covers(shape, x, y):
    kind, *a = shape
    if kind == "rrect":
        x0, y0, x1, y1, r = a[:5]
        if not (x0 <= x <= x1 and y0 <= y <= y1):
            return False
        cx, cy = min(max(x, x0 + r), x1 - r), min(max(y, y0 + r), y1 - r)
        return (x - cx) ** 2 + (y - cy) ** 2 <= r * r
    if kind == "circle":
        return (x - a[0]) ** 2 + (y - a[1]) ** 2 <= a[2] ** 2
    if kind == "poly":
        pts, inside, j = a[0], False, len(a[0]) - 1
        for i in range(len(pts)):
            (xi, yi), (xj, yj) = pts[i], pts[j]
            if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
                inside = not inside
            j = i
        return inside
    if kind == "line":
        ax, ay, bx, by, w = a[:5]
        dx, dy = bx - ax, by - ay
        t = 0.0 if dx == dy == 0 else max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
        return math.hypot(x - ax - t * dx, y - ay - t * dy) <= w / 2
    raise ValueError(kind)


def bbox(shape):
    kind, *a = shape
    if kind == "rrect":
        return a[0], a[1], a[2], a[3]
    if kind == "circle":
        return a[0] - a[2], a[1] - a[2], a[0] + a[2], a[1] + a[2]
    if kind == "poly":
        xs, ys = [p[0] for p in a[0]], [p[1] for p in a[0]]
        return min(xs), min(ys), max(xs), max(ys)
    ax, ay, bx, by, w = a[:5]
    return min(ax, bx) - w, min(ay, by) - w, max(ax, bx) + w, max(ay, by) + w


def sheet(path, faces=None, sheet_w=None, sheet_h=None, px_per_mm=None):
    FACES_, SW, SH, PX = faces or FACES, sheet_w or SHEET_W, sheet_h or SHEET_H, px_per_mm or PX_PER_MM
    W, H = int(round(SW * PX)), int(round(SH * PX))
    img = [[list(PAPER) for _ in range(W)] for _ in range(H)]
    for shapes, _, _, (ox, oy) in FACES_:
        for sh in shapes:
            x0, y0, x1, y1 = bbox(sh)
            px0, py0 = max(0, int((x0 + ox) * PX)), max(0, int((y0 + oy) * PX))
            px1, py1 = min(W, int((x1 + ox) * PX) + 2), min(H, int((y1 + oy) * PX) + 2)
            col = sh[-1]
            for py in range(py0, py1):
                row = img[py]
                for px in range(px0, px1):
                    hits = 0
                    for sy in range(SS):
                        for sx in range(SS):
                            if covers(sh, (px + (sx + 0.5) / SS) / PX - ox, (py + (sy + 0.5) / SS) / PX - oy):
                                hits += 1
                    if hits:
                        a = hits / (SS * SS)
                        p = row[px]
                        for i in range(3):
                            p[i] = round(p[i] * (1 - a) + col[i] * a)
    raw = b"".join(b"\0" + bytes(v for p in r for v in p) for r in img)
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "--side-steps":
        # one image per review fix: <prefix>-1.png … -5.png (cumulative)
        for st in range(1, 5):
            sheet(f"{sys.argv[2]}-{st}.png", faces=[(side(RED, RED_HI, step=st), 60, 19, (3, 3))],
                  sheet_w=67, sheet_h=25, px_per_mm=14)
    else:
        sheet(sys.argv[1] if len(sys.argv) > 1 else "qudelix-parts.png")
