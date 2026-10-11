# The clip in side view — from photos and the box drawing

## How to read
- Coordinates in model mm. x: 0 — far end, 60 — key end. y: 0 — the clip's outer (top) edge.
- **Drawing** — the original box scan, `icon-modeling/refs/box-side-volume.jpg`, 46 px/mm.
- **Photos:**
  - 4ec62f48 — whole side, ~20.8 px/mm;
  - 6f41cdc1 — key end close-up;
  - 145e0bdc — the same end at an angle;
  - f0dfcf95 — clip bent open.
- **New photo 7a8f2591** (side, key at the top, clip on the left; rotated in the analysis) — the main reference for the key end.
- **Rule:** shape and shading from the photos, exact sizes from the drawing. The photos are taken slightly from above, so the clip looks thicker and the gap under it smaller.

## Images
- `photo-1-whole-side.png` — where the tips, the light part of the clip and the posts are.
- `photo-2-far-end.png` — far end, ×5.
- `photo-3-key-end.png` — the key end on two photos with different lighting.
- `photo-4-angled.png` — the key end at an angle.
- `photo-5-clip-opened.png` — clip bent open: tip and posts.
- `drawing-clip-ends.png` — both ends on the drawing.
- `model-current-a-b.png` — the model now and two fix variants (A and B).

## The spot the user flagged: clip end at the key (x 57.6–60.6; y −0.3…2.5)
Image: `spot-clip-end-key-side.png`. In a row: the flagged model, the model now, new photo 7a8f2591, photo 6f41cdc1, the drawing and proposed variant A.

What the model does now:
- the clip ends at 59.7 and is rounded underneath;
- the black block under it goes to 60 and is cut square;
- background shows in the corner between them.

What it really is (photos and drawing agree):
- **The clip neither overhangs the end nor stops short of it.** It ends exactly at the body end, x≈60.0.
- **Only the clip's top outer corner is rounded**, r≈0.4–0.5.
- **The bottom corner is square.** The clip lies directly on the dark niche and the end wall. **No gap or background in the corner.**
- The body end is one straight line from the clip top down.
- **The black niche under the clip has a square top-right corner.** On the right it is closed by the body's thin end wall (~0.3–0.4 mm, a light highlight along the edge on the photo), on top by the clip. No slant.
- On the new photo the last ~2–2.5 mm of the clip are light, like a metal cap. On 4ec62f48 they are dark, on 6f41cdc1 they catch a highlight. So the clip tip sits at a different angle (slightly bent) and catches light differently. It does not change the outline.

Conclusion: fix with variant A below.
- Clip to 60.0.
- Corner radii: (top-left 0.25, top-right 0.4, bottom-right 0, bottom-left 0.2).
- The clip's right outline merges with the end outline.
- The black block needs no change: it already runs from 1.43 to the end as a rectangle.
- Optional: a thin BODY end wall x 59.65–60.0 from y 1.43 to 4.5 — like the light edge on the photo.

## What the clip is
- A straight thin strip along the whole side. The top is straight and parallel to the body.
- Constant thickness.
- The clip itself is light, metallic grey. A thin dark line runs along its lower edge — shadow.
- Under the clip there is a see-through gap to the body. Two posts stand in the gap: around x≈7–8 and around x≈11.7–12.8.

## Far end (x≈0.4)
- **Photos:**
  - the clip end is straight, vertical, corners almost sharp;
  - the clip ends slightly before the body edge, by about 0.3 mm;
  - the last ~1.4 mm of the clip are dark, almost black, through its whole thickness.
- **Drawing:**
  - the end is straight, corners rounded a little;
  - offset from the body edge ~0.4 mm (line centres 0.37–0.41);
  - no dark tip: the drawing is line art and carries no tones.
- **Model now:** starts at 0.5, radius 0.45, almost a semicircle. Too round.
- **Needed:**
  - start x=0.4;
  - top corner r=0.25, bottom r=0.2;
  - straight end between the corners.

## Key end (x≈60)
- **Photos:**
  - the clip reaches exactly to the body end, difference no more than ~0.1 mm;
  - its end continues the body end, one line;
  - the top outer corner is rounded, r≈0.4;
  - the clip's bottom corner is square: it lies directly on the body, no gap or cut under the end;
  - the last ~1.7 mm of the clip are dark on photo 4ec62f48 and catch a light highlight on 6f41cdc1 under different light. Most likely the end is slightly bent and catches light differently. On the angled photo 145e0bdc the end looks like a dark "cube".
- **Drawing:**
  - the clip's top line curves smoothly straight into the end line, r≈0.35–0.4;
  - the clip's bottom line meets the end at a right angle;
  - there is no separate clip end — clip and body end are one line.
- **Model now:** the clip ends at 59.7 with 0.45 rounding on all sides. Background shows under the rounding. Wrong.
- **Rejected fix** (slanted black block under the clip end): also wrong. There is no slant or cut under the clip end. The clip just reaches the end.
- **Needed:**
  - end x=60.0, exactly at the body end;
  - top-right corner r=0.4;
  - bottom-right corner r=0, square;
  - the clip's right outline coincides with the end outline (60.0–60.09).

## Thickness, top, bottom
- **Drawing:** 1.15 mm between line centres. Top and bottom straight, parallel to the body.
- **Photos:** look thicker (1.4–1.8 mm) and the gap smaller (2.1 mm vs 3.0). That is the angle: the top face is visible.
- **I take the drawing.** The model already does this: y 0.33–1.43, gap to the body 1.43–4.5. Don't touch.
- The clip skew on the coloured scan (0.5 mm down toward the key) is a scan artefact. On the original drawing and on the photos the clip is parallel to the body.

## Where photos and drawing disagree
| detail | photo | drawing | I take |
|---|---|---|---|
| far-end offset | ~0.3 | ~0.4 | 0.4 (drawing is more precise) |
| dark tips (1.4 and 1.7 mm) | yes | no | optional: variant B |
| clip thickness and gap | 1.4–1.8 / 2.1 | 1.15 / 3.0 | drawing |
| key end flush, no cut | yes | yes | — |

## Code to paste (variant A, from the drawing)
Replace the clip line `R(0.5, 0.33, 59.7, 1.43, …, CLIP_SCAN)` with a polygon that has different corner radii:
```python
def rpoly(x0, y0, x1, y1, r):            # r = (top-left, top-right, bottom-right, bottom-left)
    pts = []
    for cx, cy, rad, a0 in [(x0 + r[0], y0 + r[0], r[0], 180), (x1 - r[1], y0 + r[1], r[1], 270),
                            (x1 - r[2], y1 - r[2], r[2], 0), (x0 + r[3], y1 - r[3], r[3], 90)]:
        for k in range(9):
            a = math.radians(a0 + 90 * k / 8)
            pts.append((m(cx + rad * math.cos(a)), cy + rad * math.sin(a)))
    return pts
r = (0.25, 0.4, 0.0, 0.2)
s += [("poly", rpoly(0.4 - o, 0.33 - o, 60.0 + o, 1.43 + o, tuple(v + o if v else 0 for v in r)), OUTLINE),
      ("poly", rpoly(0.4, 0.33, 60.0, 1.43, r), CLIP_SCAN)]
```
For mirror: `rpoly`'s corners are mirrored via `m()`. The point order flips under the mirror; that doesn't matter for a fill.

**Variant B** (from the photos): the same, plus dark tips with the same outline:
- x 0.4–1.8 with rounded left corners;
- x 58.3–60.0 with a rounded top-right corner;
- colour about (0x22,0x22,0x26): darker than the clip, but lighter than the black block under it.

Both variants are shown in `model-current-a-b.png`.

## In passing: the posts in the gap (the user's decision differs from the photos)
- Photos 4ec62f48 and f0dfcf95 (clip bent open) agree:
  - **the post nearer the far end (x≈7–8) is metal, light, and grows from the clip**;
  - **the further one (x≈11.7–12.8) is black and grows from the body**.
- The model has it the other way round: x 6.7–8.0 black, x 11.0–12.4 grey.
- The old model `side()` (step 4) was built from the same photo and had this right: CLIP_TAB at 8.2–9.8 and POST_BLACK at 13.4–14.4.
- Let the user decide whether to change it.
