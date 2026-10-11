# Bottom (USB-C end): checking bottom_new (v1) against the drawing — v2

## How I checked
- The drawing `refs/box-bottom-usbc.jpg` (no EXIF rotation) was straightened: skew −0.93°.
- Lines converted to model mm:
  - x = (px − 154) / 69.0;
  - y = 0.33 + (px − 89) / 69.0;
  - scale 69.0 px/mm — from the 17.98 mm height (clip top → bottom), as on the side.
- Each line's position was measured at its centre (cross sections). Overlay: cyan — drawing lines, yellow — their centres.
- **Photos:** none shows the USB-C end head-on. On e4de48f3 it is visible obliquely. It confirms:
  - end-face height from the bottom to the step top: ~14.7 mm (model 14.95);
  - dark gap ~2.0 mm (model 1.93);
  - clip ~1.1 mm;
  - a separate piece at the lower edge — the seam exit.
- Key protrusion, "=" and the grilles are not visible on the photo. I take them from the drawing.

## Images
- `overlay-v1.png` — drawing over the current `bottom_new` (not copied into this folder).
- `overlay-v2.png` — drawing over `bottom_new_v2` below.
- Zooms (left v1, right v2):
  - `zoom-gap-clip.png`
  - `zoom-key-seam-right.png`
  - `zoom-usb-circles.png`

## Summary: v1 matches overall
All the big things are in place within ~0.1 mm:
- end-face outline;
- clip;
- gap;
- Π seam;
- USB-C: centre 15.14 / 7.6, outline 9.46×2.97, tongue by line 7.61×1.32;
- grilles: centres 5.0 and 25.33, y 7.58, Ø 4.5;
- keys;
- seam exits.

Minor things remain, in decreasing order below.

## Residual mismatches
1. **Clip and gap slightly narrower than the body** — ~0.2 mm on the left and ~0.15 on the right.
   - On the drawing the clip's vertical ends and the gap's side walls are at x≈0.2 and x≈W−0.15, while the body runs 0 to W.
   - In v1 the clip is full width and the gap has no outline at its edges.
   - **v2:** clip and gap at x 0.2…W−0.15; the gap gets a thin outline (`R`, r 0).
2. **The "=" springs are shifted and a bit wide.**
   - Drawing: left 2.81–4.75 (centre 3.8, width 1.95), right 25.78–27.65 (centre W−3.7). Cross line at y 2.32.
   - v1: centres 3.6 and W−3.9, width 2.2.
   - **v2:** centres 3.8 and W−3.7, width 1.95, upper tongue 1.43–2.3, lower 2.4–3.36.
3. **Dividers in the gap** — on the drawing 8.12 and 22.28, symmetric about the middle 15.2; in v1 8.0 and 22.1.
   - **v2:** middle ± 7.08.
4. **Width** — 30.38 by the drawing, 30.5 in v1. **v2:** W=30.4.
   - Everything that depends on W moves by itself.
   - USB-C goes to W/2 − 0.05, the right grille to W−5.05.
5. **Π seam:**
   - The drawing's side lines are at x 1.38 and W−1.45; v1 has 1.45 and W−1.55. **v2:** 1.40 and W−1.45.
   - The seam's top corners are rounded on the drawing, about r 0.8–1.0 at the line centre. Per the "one small radius" rule v2 rounds them **r 0.25** (sharp in v1).
   - If the user wants it like the drawing — set q = 0.9.
6. **Volume key (right)** — on the drawing y 8.4–12.1, on the side 8.0–11.9. A 0.3 mm difference.
   - Power (left) on the drawing 8.3–12.1, matches.
   - **v2 keeps Volume as on the side (8.0–11.9)** — so it meets the side face. If the end-face drawing matters more, set 8.35–12.1.
   - Protrusion: Power 1.24 (v1 1.2), Volume 1.06 (v1 1.0). Refined in v2.
7. **End-face top (step top)** — 3.23 on the drawing, 3.36 on the side. **I keep 3.36** to match the side; a 0.13 difference.
8. **Seam exits to the edges** — on the drawing the line centre is y≈16.87–17.0, on the side the seam band is 16.45–16.95 (centre 16.7). **I keep the side's**, otherwise the seam won't meet at the edge.
9. **Clip bottom** — 1.49 on the drawing, 1.43 on the side. I keep 1.43, as on the side.

Unchanged:
- body, clip and key corners — r 0.25, as on the drawing;
- USB-C "stadiums" with semicircular ends;
- grilles from the drawing.

## bottom_new_v2 — full code
```python
def bottom_new_v2(W=30.4, r=0.25):
    """USB-C end — v2 after overlaying the box drawing (line centres) in model mm."""
    o = 0.09

    def R(x0, y0, x1, y1, rad, c):
        return [rr(x0 - o, y0 - o, x1 + o, y1 + o, rad + o, OUTLINE), rr(x0, y0, x1, y1, rad, c)]

    def grille(cx, cy):                                  # black disc with 6 + 6 holes
        g = [circle(cx, cy, 2.25 + o, OUTLINE), circle(cx, cy, 2.25, HOLE)]
        for k in range(6):
            a = math.radians(90 + 60 * k)
            px, py = cx + 1.3 * math.cos(a), cy + 1.3 * math.sin(a)
            g.append(line(px - 0.2 * math.sin(a), py + 0.2 * math.cos(a), px + 0.2 * math.sin(a), py - 0.2 * math.cos(a), 0.34, RING))
            b = math.radians(60 * k)
            g.append(circle(cx + 0.62 * math.cos(b), cy + 0.62 * math.sin(b), 0.12, RING))
        return g

    cx0 = W / 2
    xl, xr = 0.2, W - 0.15                               # clip + gap: a little narrower than the body (drawing)
    s = []
    s += R(-1.24, 8.3, 0.3, 12.1, r, BLUE)               # Power (left), sticks out 1.24
    s += R(W - 0.3, 8.0, W + 1.06, 11.9, r, RED)         # Volume (right), sticks out 1.06; height as on the side
    s += R(0, 3.36, W, 18.31, r, BODY)                   # end face (from the far-end step height down)
    s += R(xl, 1.43, xr, 3.36, 0, SHADOW)                # dark gap, outer edges outlined
    for pc in (3.8, W - 3.7):                            # "=": clip tongue over a body lug, 1.95 wide
        s += R(pc - 0.97, 1.43, pc + 0.97, 2.3, 0.1, CLIP_SCAN) + R(pc - 0.97, 2.4, pc + 0.97, 3.36, 0.1, JACK)
    s += [line(cx0 - 7.08, 1.43, cx0 - 7.08, 3.36, 0.12, OUTLINE),
          line(cx0 + 7.08, 1.43, cx0 + 7.08, 3.36, 0.12, OUTLINE)]   # centre-part edges
    # seam Π: across at 4.75, down at 1.40 / W-1.45, corners r 0.25, exits at the side seam height
    J, sw, a, b, q = JACK, 0.3, 1.4, W - 1.45, 0.25
    s += [line(a + q, 4.75, b - q, 4.75, sw, J), line(a, 4.75 + q, a, 16.7, sw, J), line(b, 4.75 + q, b, 16.7, sw, J)]
    for ccx, sx in ((a + q, -1), (b - q, 1)):
        pts = [(ccx + sx * q * math.sin(t), 4.75 + q - q * math.cos(t)) for t in [k * math.pi / 16 for k in range(9)]]
        s += [line(*pts[i], *pts[i + 1], sw, J) for i in range(8)]
    s += [rr(0, 16.45, a + sw / 2, 16.95, 0, J), rr(b - sw / 2, 16.45, W, 16.95, 0, J)]
    cx, cy = cx0 - 0.05, 7.6                             # USB-C (drawing: 15.14 of 30.38)
    s += [rr(cx - 4.73, cy - 1.49, cx + 4.73, cy + 1.49, 1.49, RING),
          rr(cx - 4.38, cy - 1.14, cx + 4.38, cy + 1.14, 1.14, HOLE),
          rr(cx - 3.5, cy - 0.38, cx + 3.5, cy + 0.38, 0.38, RING)]
    s += grille(5.0, 7.58) + grille(W - 5.05, 7.58)
    s += R(xl, 0.33, xr, 1.43, r, CLIP_SCAN)             # clip
    return s
```
The code was checked by rendering: `overlay-v2.png` is drawn with it.

## What still cannot be checked without a photo of the end
- What the two circles are (grilles per the drawing).
- What the central part of the gap between the dividers is.
- The tone of the "=" springs end-on.
