# Bottom (USB-C end) from photos — v3

## Sources and how I measured
- **Main — 4 near head-on photos of the end:** 47d7e963, 11c15aa4, a4c95a21, f008428b. Rotate them by EXIF first (orientation 6); then the clip is on top.
- **Rectification.** Each photo was rectified by the end face's four corners into model mm: x 0…30.4, y 3.36…18.31 (from the step top to the bottom, as on the side).
- **Best frames:** 11c15aa4 (almost frontal) and f008428b. a4c95a21 and 47d7e963 have strong perspective; I used them only to check "what is there".
- **The drawing** only for exact positions where the photos are unclear: keys, divider 8.1.
- **The parts in the gap sit deep inside the body.** So they "move" from frame to frame by several mm — parallax. Positions come from the most frontal frame (11c).

## Images
`photo-v2-v3.png`:
- top: rectified photos 11c and f00;
- bottom: v2 and v3;
- all at the same scale, 1 mm grid.

## What the photos showed, and what changed vs v2
1. **Screws, not grilles.** Each screw is:
   - a dark recess (counterbore) Ø≈3.5–3.7 with a light highlight along its rim;
   - inside, a metal head Ø≈2.6;
   - a hex socket Ø≈0.7 in the centre.
   - Centres (4.5–5.0; 7.75–7.9). I take (4.7; 7.75) and (W−4.7; 7.75), level with the USB-C.
   - The drawing showed them as black Ø4.5 discs. It is simplified: the photos win.
2. **USB-C:**
   - a recess with a chamfered rim, 10.1×3.5, centre (15.15; 7.85);
   - a dark opening inside, ~8.9×2.5;
   - the tongue in the upper half, y≈7.4–7.85, length ~7.3.
   - v2 had 9.5×3.0 at y 7.6 — on the photos the connector is slightly bigger and slightly lower.
3. **The gap under the clip is SEE-THROUGH at the sides** (background shows), not dark.
   - In the middle a dark block x 2.0–22.3. Its left part 2.0–8.0 is darker and deeper — the drawing has a divider at 8.1 there.
   - At the block's right end a small rounded lug x 21.6–23.3, from 1.6 down to the end-face top.
   - Right of the lug (23.3–30.2) and left (0.2–2.0) — see-through.
4. **Spring contacts "||||":**
   - visible on f00 on the left, x≈2.6–5, against a dark background;
   - on a4c (strong angle) visible on the right;
   - on the most frontal 11c almost invisible.
   - In v3 they are on the left, in place of the drawing's left "=" (x 3.0–4.5), as thin light strokes.
   - The drawing's right "=" is not visible head-on: it is see-through there.
5. **Inset panel (Π seam):**
   - a groove across the top at y≈4.9;
   - on both sides about 1.0–1.4 mm from the edge (1.15 on average over two frames);
   - at the bottom the grooves end at y≈17.0 with a short tick outward to the edge.
   - On the photos it is a dark groove with a light highlight on one side, not a black band.
   - v2 had 4.75 and 1.40.
6. **Clip:**
   - a thin plate y 0.3–1.4, across roughly 0.2–0.6 … 30.1–30.3;
   - on two frames the edges "wander" by ±0.3 due to parallax: the clip is not in the end-face plane;
   - no clear shift to one side on the frontal frame. I take 0.4 … W−0.25.
7. **Keys** are almost invisible on the frontal photos: fingers and the angle hide them. On a4c a tab is visible on the right. Sizes kept from the drawing and the side: Power on the left sticks out 1.24, Volume on the right 1.06.
8. **Corners** — one small radius, 0.25 (body, clip, keys). On the photos the end-face corners are a bit rounder, ~0.4–0.5, but the user's rule wins.

## Colours (existing constants only)
| part | colour |
|---|---|
| end face | BODY |
| block in the gap | BODY, left part SHADOW |
| lug | BODY_HI |
| contacts | STEEL |
| see-through | SEE_THROUGH |
| grooves | HOLE, highlight BODY_HI |
| screw counterbore | HOLE, rim BODY_HI |
| screw head | RING / STEEL |
| hex socket | HOLE |
| USB-C | rim BODY_HI, recess OUTLINE, opening HOLE, tongue RING |
| clip | CLIP_SCAN |

## bottom_new_v3 — full code
Checked by rendering, see the bottom-right panel of the image.
```python
def bottom_new_v3(W=30.4, r=0.25):
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
    s += [line(3.0 + 0.5 * k, 2.2, 3.0 + 0.5 * k, 3.0, 0.18, STEEL) for k in range(4)]   # spring contacts "||||" (f00; drawing "=" at 2.8–4.75)
    s += R(21.6, 1.6, 23.3, 3.36, 0.5, BODY_HI)          # small rounded lug at its right end
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
```

## What is still imprecise
- **The deep parts in the gap** (block, lug, contacts) depend on the angle. Taken from the most frontal frame.
- **Key height** — from the drawing and the side: the keys are not visible on the photos.
