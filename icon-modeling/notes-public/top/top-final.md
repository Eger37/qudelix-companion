# Top (jack end) — final check against the photos

## How I checked
- **Photos:** `jack-end-headon-03575498` (main) and `jack-end-1-41494079` rectified by the end face's corners into model mm (W 30.4). The outlines of `top_new(fx=('springs2',))` were laid over them.
- **Others:** `jack-end-2/3` and the drawing `box-top-jacks-1fe8c8d1` (rotated 180°, clip at the bottom) — to check shapes.
- **Allowing for the angle:** both photos show the left side wall. So recessed parts (niche floors, springs, jack openings) shift by 0.3–0.7 mm. Where the photos disagree I take the average or the drawing.
- **The user's rules:** no dust, radius 0.25, identical springs.

## Image `top-final.png`
- photo and drawing;
- outlines of the current and the fixed model over photo 035;
- both models;
- the fixes over photo 41494079;
- both models at icon size (~32 px).

## Mismatches (most visible first)
1. **The jacks are too small — `jacks2`.**
   - **Big (3.5):** on the photo a light raised bezel out to r≈3.8–3.95 (over two frames), opening r≈2.75, centre (14.9; 6.3).
     - Model: bezel r 3.32–3.5, opening 2.3, centre y 6.7.
     - The bezel is ~0.4 too small, the opening ~0.45, the centre 0.4 too low.
   - **Small (2.5):** bezel r≈2.85, opening r≈1.6–1.8, centre y≈12.95 (model 2.55 / 1.7 / 13.1).
   - **Inside the big jack the photo shows a brass (gold) ring** r≈1.55. The model has grey STEEL. It is the end face's only warm detail; worth keeping.
     - Needs a new constant `BRASS = (0xB8, 0x98, 0x5A)`. There are no existing warm colours.
   - **x centres:** on the photo the small jack is ~0.6 left of the big one. But on the drawing they are exactly one above the other, and this is the angle (the small one is recessed deeper). **I keep both at W/2.**
2. **The gap between the jacks — `bridge`.**
   - On the photo and the drawing the bezels **touch**, making a "figure 8". There is a small bridge between them; on the photo a light bump near (14.9; 9.9).
   - The model has a 0.2 mm gap; the bezels don't touch.
   - With `jacks2` the bezels touch (big bottom 10.1, small top 10.1). `bridge` adds a narrow 0.9 mm bridge.
3. **The key ears sit in a notch in the end face — `earnotch`.**
   - On both photos the key sits in a rectangular notch in the face. On the key's inner side a dark slit shows, ~0.6 mm (left) and ~1.3 mm (right). Notch: left x 0…2.8, right W−3.8…W.
   - Key: left x −1.3…2.2, y 6.0…10.0; right W−2.5…W+1.1, y 6.05…10.4.
   - Model: the key reaches 3.0 mm onto the face with no slit.
   - The outward protrusions (1.3 / 1.1) and the groove along the wall line (x 0 / W) match.
4. **Side-wall lines — `sidewall`.**
   - On the drawing there is a double line along the left and right edges. On the photo, on the right, a light edge band ~0.9–1.0 mm from the edge, from the seam to the niche; the ears interrupt it.
   - The model has no such lines.
   - Fix: thin BODY_HI lines at x 0.9 and W−0.9.
5. **Mic — `mic2`.**
   - Photo: (7.25…7.9; 11.4…12.0), a hole in a small dimple r≈0.55. Drawing: left of the small jack, level with its top.
   - Model: (8.6; 12.1) — ~1 mm to the right.
   - Fix: (7.6; 11.75) + a light dimple rim.
6. **Seam at the front edge — `seam2`.**
   - Photo: a line at y≈1.1. Drawing: 1.25.
   - Model: 1.5.
   - Fix: 1.25.
   - On the side the seam band gives y_t 1.36–1.86 — a small difference from the side, but on the end face the photo and the drawing agree on ~1.2.
7. **Strut on the niche floor — `strut2`.** Photo: a light band ~0.65 mm (16.25…16.9). Model: 0.45.
8. **Tongue and clip pieces — `clip2`.**
   - Tongue edges on the photo x 10.0 and 19.4 (model 10.3 / 19.6).
   - The right clip piece ends at ≈29.7 on both photos (model W−0.2 = 30.2).
   - Medium confidence: the clip is not in the end-face plane, so the angle matters more here.
9. **What matches:**
   - niche top (13.6), right niche edge (30.0–30.2), tongue bottom;
   - springs: 4 identical turns, mirrored, 4.8–7.6 / W−7.6…W−4.8, y 14.0–15.3;
   - ear heights ±0.3.
10. **Corners.** On the photo the end-face corners are ~0.5–0.8, but by the user's rule they stay 0.25. The concave corners between tongue and niche are rounded ~0.5 on the photo. Rule 0.25 — not changed.
11. **Colours.**
    - The jack bezels are lighter than the body on the photo — RING fits.
    - Niches — SHADOW, correct.
    - No dust or crumbs in the niches.
    - The only new one — BRASS (see item 1).

## What survives at icon size (48 dp ≈ 32 px, 1 px ≈ 1 mm)
- **Will vanish:**
  - springs;
  - struts on the niche floors;
  - wall lines;
  - the seam at the edge;
  - the grooves on the ears;
  - the bridge between the jacks;
  - the small jack's contact dot;
  - the mic (0.6 mm < 1 px);
  - the slits at the ears (0.6–1.3 mm — borderline).
- **Will survive and worth emphasising:**
  - **the "figure 8" of the two jacks** — the main recognisable mark. With `jacks2` the bezels are bigger and touch, which reads better;
  - **the brass ring** — a warm dot in the centre, visible even at 2–3 px;
  - **the red and blue ears at the sides**;
  - **the dark niches with the tongue in the middle** — a "Π" silhouette at the bottom;
  - the light band of the clip.
- **For an icon variant:** no small stuff (springs, struts, wall lines, seam). The jack bezels can be a shade lighter (STEEL instead of RING) so the figure 8 doesn't merge with the body.

## Code to paste
The full `top_new` with the fixes, each enabled by its own fx:
- `jacks2`, `bridge`, `mic2`, `earnotch`, `sidewall`, `seam2`, `strut2`, `clip2`;
- springs always `turns` (as `springs2`);
- with no fx — as now.

`BRASS` is a new constant; put it next to RING / STEEL. The code was checked by rendering: the right column of `top-final.png`, all fx on.
```python
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
    st = 0.65 if 'strut2' in fx else 0.45                             # strut on the niche floor (photo: ~0.65)
    s += R(0.6, yc - st, tx0, yc, 0.1, STRUT) + R(tx1, yc - st, W - 0.6, yc, 0.1, STRUT)
    fO, fF = R(0, 0, W, yb, r, BODY)
    tO, tF = R(tx0, yb - 1.0, tx1, yo, r, BODY)
    s += [tO, fO, tF, fF]
    sy = 1.25 if 'seam2' in fx else 1.5                               # seam near the front edge (photo 1.1, drawing 1.25)
    s += [line(0, sy, W, sy, 0.3, JACK)]
    if 'sidewall' in fx:                                              # side-wall edge lines ~0.9 in from both edges (drawing double lines + lit band on photos), broken by the key ears
        for xw in (0.9, W - 0.9):
            s += [line(xw, sy + 0.4, xw, 5.6, 0.12, BODY_HI), line(xw, 10.8, xw, yb - 0.5, 0.12, BODY_HI)]
    jx = W / 2
    if 'jacks2' in fx:                                                # photos: wider raised bezels, bigger openings, brass ring in the big jack
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
```
