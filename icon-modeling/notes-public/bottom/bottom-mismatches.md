# USB-C end (bottom): drawing, photos and model

## Sources
- **Drawing:** `refs/box-bottom-usbc.jpg`. Use it WITHOUT the EXIF rotation: raw, the text reads upright.
- **Photos:** no photo shows the USB-C end head-on. It appears obliquely only on e4de48f3 (top of the frame): its edges, the seam at the lower edge, the dark gap and the clip are visible. The other photos show the side and the key end. 1c097907, 1fe8c8d1, 768abbf2, edecaece are photos of the box.
- **So sizes and details come from the drawing.** Scale: the height from the clip top to the bottom = 17.98 mm, as on the side (68.9 px/mm in the original). The photo confirms three things: the seam at the lower edge, the dark gap and the clip.
- **Needed from the user:** a head-on photo of the USB-C end. The main question is what the two circles are.

## Orientation
- On the drawing the end is seen from outside, clip on top.
- **The Volume key is on the right**, Power on the left. This agrees with both the geometry and the protrusion heights.
- **In the model the clip is at the bottom — that is upside down.** In the unfolded layout the bottom sits under the back face, and the shared edge is the clip side. So the clip must be on top, as on the drawing.
- Coordinates below are as on the drawing: x from the body's left edge, y as on the side (0 — clip top, 18.31 — bottom). To keep the clip at the bottom, rotate by 180°: x' = W − x, y' = 18.31 − y. Do not mirror.

## Images
- `drawing-model-proposal.png` — drawing, current model and proposal at the same scale, 1 mm grid.
- The circles close-up (`circles-drawing-model.png`) was not copied into this folder.
- `photo-end-oblique.png` — the only photo where the end is visible at all.

## Mismatches (most important first)

### 1. Proportions and heights don't match the side
| | drawing (= side) | model now |
|---|---|---|
| clip | y 0.33–1.43 (1.1 mm) | plate 2.0 mm |
| gap under the clip | 1.43–3.36 (1.9 mm) | ~1.8 mm, but overlapping |
| body end face | 3.36–18.31 (15.0 mm) | 13.4 mm |
| total height | 17.98 | 16.6 |
| width | **≈30.5** | 32 |

- The body end face starts right under the gap, at y 3.36 — the height of the far-end "step" on the side. The step runs across the full width, so the end face is taller than the body on the side (4.5…18.31).
- **The drawing gives a width of 30.5 mm**, while the model and the back face use 32. Check with a ruler: either the width is 30.5, or, if it is 32, the whole thickness is ~18.9 and the whole side must be rescaled. For now I take the drawing.

### 2. The seam is a Π-shaped frame, not a U
- **Drawing:**
  - the line runs across the top at y≈4.75 (the level of the body top on the side, 4.5);
  - on both sides it goes down 1.45–1.55 mm in from the side edges;
  - at the bottom, at y≈16.9, it turns OUTWARD to the edges and meets the side seam (16.45–16.95);
  - **no line across the bottom**: the bottom strip and the middle of the end are one part.
- **Model:** U (down the sides, across the bottom at 12.3, nothing on top). The opposite.
- **Photo e4de48f3:** at the end's lower edge a separate piece, separated by a dark seam line ~1.1–1.5 mm from the bottom. That is the seam exit to the edge.
- **Do** (colour as the side seam, JACK; width 0.3):
  - across: (1.45; 4.75)–(W−1.55; 4.75);
  - down: x=1.45 and x=W−1.55, from 4.75 to 16.7;
  - exits to the edge: bands y 16.45–16.95 from the edge to the seam, both sides.

### 3. The two circles are not torx screws
- **Drawing:** a solid black disc Ø≈4.4–4.5 mm.
  - Inside, 6 large oblong holes on a circle r≈1.3: ~0.5×0.33 mm, elongated along the circle.
  - Another 6 small ones on a circle r≈0.62: Ø≈0.25. Dark in the centre.
  - This is a grille (perforation), not a drive: a torx would be a star-shaped recess, not separate holes.
- **Where:** centres (5.0; 7.56) and (W−5.2; 7.56) — at the height of the USB-C middle.
- **Model:** a Ø3.5 "torx" with a light ring and a star, centres (5.2 / 26.8). Smaller and different.
- **What they really are cannot be seen on the photos.** They look like grilles (mic/vent). Ask the user.
- **Do:** a black disc r 2.25 with a thin outline, 6 light slot-holes at r 1.3 and 6 dots r 0.12 at r 0.62. Code `grille()` below.

### 4. The keys stick out of the sides
- **Drawing:** two tabs at the end's edges — the side keys seen from the end.
  - **Right, Volume:** y 8.0–11.9 (exactly as the key on the side), sticks out ≈1.0 mm.
  - **Left, Power:** y≈8.3–12.1, sticks out ≈1.2 mm.
  - Each has a second line inside: the key cap.
- **Model:** no tabs.
- **Do:** `rr(W−0.3; 8.0; W+1.0; 11.9; r; RED)` and `rr(−1.2; 8.3; 0.3; 12.1; r; BLUE)`, with an outline.

### 5. The clip end-on
- **Drawing:**
  - a plate across the full body width, y 0.33–1.43 (1.1 mm);
  - small corners;
  - no separate "hook" bent toward the body.
- **Model:** a 2.0 mm plate at the bottom, r 0.7, and a hook 10.8–21.2 reaching into the gap.
- **Do:** `R(0, 0.33, W, 1.43, 0.25, CLIP_SCAN)`.

### 6. The gap between clip and end face
- **Drawing:** the row y 1.43–3.36 is split by vertical lines into cells:
  - at the edges (x 0–2.5 and 27.6–30.5) — empty;
  - spring tabs "=": x 2.5–4.7 and 25.5–27.6 (centres 3.6 and 26.6);
  - dividers at x≈8.0 and 22.1, with a long central cell 8.0–22.1 between them.
- **Photo e4de48f3:** the gap is dark from the end.
- **Model:** a dark band (SHADOW) — correct. But it has coil springs and a hook in it.
- **Do:**
  - gap background SHADOW;
  - the central part 8.0–22.1 separated by two thin lines (OUTLINE, 0.12). What it is (maybe the middle of the clip end) cannot be told without an end photo. Don't invent a hook.

### 7. The "springs" are two tabs one above the other, not coils
- **Drawing:** each "spring" is drawn as "=" 2.2 mm wide: two rectangles one above the other.
  - **upper** y 1.43–2.3 hangs from the clip;
  - **lower** y 2.45–3.36 grows from the end face.
- **Side photos** (f0dfcf95, 4ec62f48): these are two flat tabs — a metal one from the clip and a black one from the body. End-on they line up across the width. No coils.
- **Model:** five slanted strokes (a coil) at 6.4 and 24.0.
- **Do:** at x 3.6±1.1 and (W−3.9)±1.1:
  - upper `R(…, 1.43, …, 2.3, 0.1, CLIP_SCAN)`;
  - lower `R(…, 2.45, …, 3.36, 0.1, JACK)`.

### 8. USB-C — sizes
- **Drawing:** centre (15.15; 7.6).
  - Outer "stadium" 9.5×3.0 mm.
  - Inner (tongue) by outline 7.6×1.3, without the line ~7.0×0.75.
- **Model:** ring 9.4×3.4, opening 8.4×2.5, tongue 5.2×0.6. The model is 0.4 mm taller, and the tongue ~1.8 mm shorter.
- **Do:**
  - ring `rr(cx−4.75, cy−1.5, cx+4.75, cy+1.5, 1.5, RING)`;
  - opening `rr(cx−4.4, cy−1.15, cx+4.4, cy+1.15, 1.15, HOLE)`;
  - tongue `rr(cx−3.5, cy−0.38, cx+3.5, cy+0.38, 0.38, RING)`.

### 9. Corner radius
- On the drawing the end-face and clip corners are small: ~0.25–0.3 mm.
- **The user's rule (one small radius, 0.25) fits here:** body, clip and keys — r 0.25. Model now: body 1.2, clip 0.7.
- The USB-C "stadiums" keep semicircular ends; that is part of the connector's shape.

## Proposal code (as in the "proposal" image)
```python
def bottom_new(W=30.5, r=0.25):
    s = []
    s += R(-1.2, 8.3, 0.3, 12.1, r, BLUE)          # Power (left)
    s += R(W-0.3, 8.0, W+1.0, 11.9, r, RED)        # Volume (right)
    s += R(0, 3.36, W, 18.31, r, BODY)             # end face, from the step height to the bottom
    s += [rr(0.15, 1.43, W-0.15, 3.36, 0, SHADOW)] # dark gap
    for cx in (3.6, W-3.9):                        # "=": clip tongue over a body lug
        s += R(cx-1.1, 1.43, cx+1.1, 2.3, 0.1, CLIP_SCAN) + R(cx-1.1, 2.45, cx+1.1, 3.36, 0.1, JACK)
    s += [line(8.0, 1.43, 8.0, 3.36, 0.12, OUTLINE), line(22.1, 1.43, 22.1, 3.36, 0.12, OUTLINE)]
    s += [line(1.45, 4.75, W-1.55, 4.75, 0.3, JACK), line(1.45, 4.75, 1.45, 16.7, 0.3, JACK),
          line(W-1.55, 4.75, W-1.55, 16.7, 0.3, JACK),
          rr(0, 16.45, 1.6, 16.95, 0, JACK), rr(W-1.7, 16.45, W, 16.95, 0, JACK)]   # seam: Π + exits to the edges
    cx, cy = 15.15, 7.6                            # USB-C
    s += [rr(cx-4.75, cy-1.5, cx+4.75, cy+1.5, 1.5, RING), rr(cx-4.4, cy-1.15, cx+4.4, cy+1.15, 1.15, HOLE),
          rr(cx-3.5, cy-0.38, cx+3.5, cy+0.38, 0.38, RING)]
    s += grille(5.0, 7.56) + grille(W-5.2, 7.56)
    s += R(0, 0.33, W, 1.43, r, CLIP_SCAN)         # clip, full width
    return s

def grille(cx, cy):                                # black disc with 6 + 6 holes
    s = [circle(cx, cy, 2.25 + o, OUTLINE), circle(cx, cy, 2.25, HOLE)]
    for k in range(6):
        a = math.radians(90 + 60 * k)
        px, py = cx + 1.3 * math.cos(a), cy + 1.3 * math.sin(a)
        s.append(line(px - 0.2 * math.sin(a), py + 0.2 * math.cos(a), px + 0.2 * math.sin(a), py - 0.2 * math.cos(a), 0.34, RING))
        b = math.radians(60 * k)
        s.append(circle(cx + 0.62 * math.cos(b), cy + 0.62 * math.sin(b), 0.12, RING))
    return s
```
- `R(...)` here is a rectangle with a thin outline o=0.09, as in `side_ideal`.
- Panel on the sheet: width W+2.2 (keys), height 18.4. Move its position in the layout for the new height.

## In passing
- **Back face `back()`:** RED (Volume) is drawn on the left at x −0.7. By geometry, looking at the clip side from outside with the key end up, Volume should be on the right. Check whether the back face is mirrored.
- **Its width is 32**, while the bottom drawing gives 30.5 — see item 1.
