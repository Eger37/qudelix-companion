# Back (clip side) — back_new

## What the photos and drawings have
All 20 photos (EXIF-rotated) and 4 box drawings were reviewed.
- **There is no head-on photo of the back.** No photo shows the clip's outer surface as a whole. The box has no back drawing either: only two sides and two ends.
- **What is visible indirectly:**
  - **145e0bdc** — the key side at an angle. The clip is visible only edge-on: a thin plate of constant thickness, its end at the key end exactly flush with the end.
  - **47d7e963, 11c15aa4, a4c95a21, f008428b** (USB-C end) — the clip's edge and a strip of its outer surface at a steep angle. The plate is smooth, flat, grey-metallic, full width (x 0.4…W−0.25). Corners rounded, ~0.4–0.5. No pictures, slots or lettering on the visible edge.
  - **03575498, 41494079** (jack end) — the clip is in two pieces. Between them the body tongue (x 10.3…19.6) reaches the clip's outer face. So at the jack end the clip has a slot, and the body shows in it.
  - **Side photos** (4ec62f48, 7a8f2591, e4de48f3) — the clip runs the whole length. At the far (USB-C) end it stops ~0.4 mm short of the end, at the key end it is flush. The last 1.4–2.5 mm catch light differently, apparently slightly bent. Only the highlight differs, so I don't draw it.

## What is seen looking straight at the back
1. **The clip covers almost everything** — CLIP_SCAN, x 0.4…W−0.25, along the length from the jack end (flush) to 0.4 mm short of the USB-C end.
2. **A narrow body rim** round the clip: ~0.25–0.4 mm at the sides and 0.4 mm at the USB-C end.
3. **A slot at the jack end** with the body tongue: x 10.3…19.6. Along the length from the end to the start of the ear on the side: side x 43.35 → back y 0…16.65. The body in the slot is flush with the clip.
4. **The key edges** stick out at the sides, along the key's length on the side (42.4…60.25 → y −0.25…17.6, i.e. at the jack end):
   - **Volume — right, sticks out 1.06**;
   - **Power — left, sticks out 1.24**.
5. **What is not there** (guessed in the old `back()` and not confirmed):
   - two slots;
   - a narrow 10.4×24 "clip" over the plate;
   - text lines and icons;
   - a light band.
   No dirt is drawn.

## Orientation in the layout
- **The jack end is at the top** (y = 0). Above it sits `top_new`, facing the back with its clip edge (its clip is at the bottom).
- **The USB-C end is at the bottom** (y = 60). Below it sits `bottom_new`, whose clip is on top.
- Seen from outside, **Volume (red) is on the right**, as on both ends. In the old `back()` it is on the left — a mistake.
- Relation to the side: y_b = 60 − x_side.

## Radii
- Body, keys and slot — **0.25**, as on the side faces.
- The clip plate — **rb = 0.5**. This is the one "other" value for the back: on the end-face photos the clip corners are ~0.4–0.5.

## back_new — code to paste
Uses only the existing `rr`, OUTLINE, BODY, CLIP_SCAN, RED, BLUE.
```python
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
```
- Panel width in the layout W+2.3 (keys), height 60.
- In `FACES` replace `(back(), 32, 60, (6, 30))` with `(back_new(), 30.4, 60, (6, 30))`. Shift the neighbouring panels for the 30.4 width if needed.

Checked by rendering: `back-photo-old-new.png` (the old `back()`, `back_new()` and the photo evidence).

## What cannot be determined from the existing photos — which photos to ask for
1. **The back (clip) straight on, as a whole** — the main one. Is there a logo, lettering, engraving, slots, embossing or ribs on the clip?
2. **The clip close-up at the jack end, from above.** How long the slot is and how it ends (rounded, straight edge). Is the body tongue flush with the clip or recessed?
3. **The clip close-up at the USB-C end, from above.** How far the end stops short of the end face, and what its edge is like (bend, chamfer).
4. **The clip edges along the long sides, from above.** Is there an even body rim at the sides (0.25–0.4), and are the keys visible from this side?
5. **The clip's colour and texture in light.** Matte metal, anodised, brushed "in one direction" — is striped shading needed?
6. **Is anything under the clip visible past its edges**: e.g. the lug 21.6–23.3 at the USB-C end (on `bottom_new`). From above it is most likely hidden, but there is nothing to confirm it.
