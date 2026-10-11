# Top (jack end) from photos and the drawing — top_new

## Sources and how I measured
- **Photos 41494079, 06a74471, 40591a8e, 03575498** — the end almost head-on, clip at the bottom. No EXIF rotation needed (orientation 1).
- **Rectification:** by the four corners of the body's end face.
  - Height: from the front edge (the bottom on the side, 18.31) to the body top over the gaps (4.5 on the side).
  - Best frame — 03575498. Second — 41494079.
  - Horizontal positions differ by ~0.5–1 mm between the two frames. Cause — the angle: part of the left side wall is visible.
- **The drawing** `refs/box-top-jacks.jpg` rotated by 180° so the clip is at the bottom, as on the photos. With this rotation it matches the photos in every detail. From the drawing I took the centre and proportions.
- **Coordinates as on the side, only flipped:** y_t = 18.31 − y_side.
  - 0 — front edge (the device's bottom);
  - 13.81 — body top (4.5 on the side);
  - 16.88 — clip underside (1.43);
  - 17.98 — clip outer face (0.33).
- **Width** W = 30.4, as for the bottom.

## Orientation
- **Clip at the bottom:** in the layout the top sits above the back face, and the shared edge is the clip side.
- **Volume on the right, Power on the left.** This follows from the side geometry and agrees with the photos: the right key's height matches Volume on the side (8.0–11.9).
- **Note:** in the current `top_end()` and `back()` RED (Volume) is drawn on the LEFT. By geometry, seen from outside, Volume should be on the right on both the top end and the back. `top_new` has Volume on the right. If the user decides otherwise, swap RED and BLUE (in `back()` too).

## Image
`photo-drawing-current-new.png`: rectified photo, drawing (rotated), the current `top_end()` and `top_new()`. 1 mm grid.

## What is really there (photo + drawing) and missing from top_end
1. **The lower part of the end is not solid.** Above the clip, on both sides, there are dark niches (that is dark block 19 on the side, seen end-on).
   - **Springs** show in the niches — short light coils.
   - On the niche floor, right on the clip, lie **struts** (the clip's mount seen end-on).
   - Niches: x 0.2…10.3 and 19.6…30.2; y_t 13.6…16.88.
2. **The body's central tongue = the ear seen end-on.**
   - The body between the niches comes down to the clip's outer face: x 10.3…19.6, down to y_t 17.98.
   - **Here the clip is in two pieces:** the ear passes through a slot in the clip.
   - The drawing shows it clearly: the clip's inner line runs only over the niches, not over the tongue.
3. **Clip** — two plates: x 0.2…10.3 and 19.6…30.2, y_t 16.88…17.98. Corners r 0.25.
4. **Jacks centred, one above the other, rings almost touching.**
   - Big (3.5 mm) — further from the clip: centre (15.2; 6.7). Ring r 3.5 (dark groove) / 3.32 (light bezel), opening r 2.3, a metal ring r 1.45 inside.
   - Small (2.5 mm BAL) — nearer the clip: centre (15.2; 13.1). Ring r 2.7 / 2.55, opening r 1.7. Its lower part reaches onto the tongue.
   - In top_end big and small are swapped and sit higher.
   - Heights from the two photos and the drawing differ by ~1 mm; the average is used.
5. **Mic** — a dot r 0.32 at (8.6; 12.1), left of the small jack. In top_end it is on the right, by the big one.
6. **Keys — "ears".** The key ends cover the side edge: ~3.0 mm reach onto the end face and they stick out by 1.24 (Power) and 1.06 (Volume), as on the bottom.
   - A **groove** crosses the key — the side-wall line. The key's inner edge is light.
   - Heights from the side: Power y_t 6.21–10.01, Volume 6.41–10.31.
   - Currently the ears are drawn outside the body and without a groove.
7. **The side seam runs across the front edge:** y_t ≈1.5 (16.8 on the side).
   - Visible both on the drawing (a band along the edge) and on the photo (a light step along the top).
   - On this end the seam is a straight line across the full width, not a Π as on the bottom.
8. **Corners** — one radius, 0.25. The body is r 1.4 now.

## top_new — full code
Checked by rendering, see the bottom-right panel.
```python
def top_new(W=30.4, r=0.25):
    """Jack end, from 4 near head-on photos (rectified) + box drawing. Clip at the BOTTOM (it meets the back
    face), Volume on the right. y_t = 18.31 - y_side: 0 = front edge, 13.81 = body top (side 4.5),
    16.88 = clip underside (side 1.43), 17.98 = clip outer edge (side 0.33)."""
    o = 0.09
    def R(x0, y0, x1, y1, rad, c):
        return [rr(x0 - o, y0 - o, x1 + o, y1 + o, rad + o, OUTLINE), rr(x0, y0, x1, y1, rad, c)]
    yb, yc, yo = 13.81, 16.88, 17.98
    tx0, tx1 = 10.3, 19.6                                 # the ear seen end-on: body tab up to the clip's outer face
    s = []
    # dark cavities under the clip at both sides, coil springs, and the struts (end-on) lying under the clip
    s += R(0.2, yb - 0.2, tx0, yc, 0, SHADOW) + R(tx1, yb - 0.2, W - 0.2, yc, 0, SHADOW)
    for sx in (3.6, 20.4):                                # coil springs seen side-on: short light bars
        s += [line(sx + 0.55 * k, 14.6, sx + 0.55 * k + 0.15, 15.9, 0.2, STEEL) for k in range(6)]
    s += R(0.6, yc - 0.45, tx0, yc, 0.1, STRUT) + R(tx1, yc - 0.45, W - 0.6, yc, 0.1, STRUT)
    # body: face + central tab (outlines first, then fills, so they merge)
    fO, fF = R(0, 0, W, yb, r, BODY)
    tO, tF = R(tx0, yb - 1.0, tx1, yo, r, BODY)
    s += [tO, fO, tF, fF]
    s += [line(0, 1.5, W, 1.5, 0.3, JACK)]                          # the side seam runs across the front edge here (drawing + photo)
    # jacks: big (3.5 SE) far from the clip, small (2.5 BAL) near it; the rings almost touch
    jx = W / 2
    s += [circle(jx, 6.7, 3.5, HOLE), circle(jx, 6.7, 3.32, RING), circle(jx, 6.7, 2.3, HOLE),
          circle(jx, 6.7, 1.45, STEEL), circle(jx, 6.7, 1.25, HOLE)]
    s += [circle(jx, 13.1, 2.7, HOLE), circle(jx, 13.1, 2.55, RING), circle(jx, 13.1, 1.7, HOLE),
          circle(jx - 0.3, 14.3, 0.18, STEEL)]                       # contact pin glint
    s += [circle(8.6, 12.1, 0.32, HOLE)]                             # mic hole
    # key ears: the key ends cover the side edge — 3.0 on the face, 1.24 / 1.06 outside (as on the bottom face)
    for x0, x1, xe, c, ch, y0, y1 in ((-1.24, 3.0, 0.0, BLUE, BLUE_HI, 6.21, 10.01), (W - 3.0, W + 1.06, W, RED, RED_HI, 6.41, 10.31)):
        s += R(x0, y0, x1, y1, r, c)
        s += [line(xe, y0 + 0.15, xe, y1 - 0.15, 0.14, OUTLINE)]   # groove: the side wall line across the key
        hx = x1 - 0.3 if xe == 0.0 else x0 + 0.3
        s += [line(hx, y0 + 0.3, hx, y1 - 0.3, 0.18, ch)]          # lit inner edge
    # clip: two pieces either side of the tab (the ear passes through a slot in the clip)
    s += R(0.2, yc, tx0, yo, r, CLIP_SCAN) + R(tx1, yc, W - 0.2, yo, r, CLIP_SCAN)
    return s
```
Panel size: width W+2.3 (ears −1.24…W+1.06), height 18.0. In the layout the top sits above the back face, with its lower (clip) edge toward it.

## What is imprecise
- **Horizontal centres** (jacks, tongue, mic) "float" by ~0.5–1 mm on the photos. Taken from the drawing: jacks centred, tongue 10.3–19.6, mic 8.6.
- **Springs and struts in the niches** are drawn approximately — by look, without exact sizes.
