# Step on the side's bottom edge — from the drawings and photos

## Short answer
- **The step is on the side's long bottom edge, at the far end.** It is not on the end face and not on the seam.
- From the far end to **x≈8.0 mm** the body's bottom edge is **~0.15 mm higher** (closer to the seam) than along the rest of the length.
- At x≈8.0 the edge steps down by 0.15 mm and then runs straight to the key end.
- The bottom strip under the seam is 0.15 mm narrower over the first 8 mm. The seam itself does not move.
- **There is no such step at the key end.**
- My earlier U-groove on the seam (`'seamstep'`) is wrong; remove it.

## Where this comes from (`evidence-drawings-and-photo.png`)
Three independent sources show the same place and the same size.

| source | where the step is (from the far end) | height | direction |
|---|---|---|---|
| drawing, Pairing/Power side (`refs/box-side-pairing-power.jpg`) | x≈8.1 | ~0.15 mm | the edge is inset toward the end |
| drawing, Volume side (`refs/box-side-volume.jpg`, photo e019bf26) | x≈7.9 | ~0.15 mm | same |
| photo e4de48f3 (bottom edge, against the light) | x≈8.6 | ~0.15 mm (7 px at 46 px/mm) | same |
| coloured scan (line centre) | between 7.3 and 8.3 | ~0.2 mm | same |

Details per source:
- On the Pairing side the step is clearest: the bottom-edge line simply shifts as a whole.
- On the Volume side the line makes a "tooth" at the same place. It also shows on the box photo e019bf26 — the right (bottom) edge at the far end.
- On photo e4de48f3 the side edge has a clear step at the level of the clip posts, on the far-end side.
- I take x=8.0 (the drawings are more precise than the photo) and a height of 0.15.

On the user's photo of the model (03ee2b7c) the bottom edge is straight — the step is missing.

## What it looks like
- The top and the seam do not change.
- The body's bottom line:
  - from the far end (with its bottom corner r 0.25) runs straight at y=18.16;
  - at x=8.0 a short vertical step down;
  - then straight at y=18.31 to the key end.
- The step's bottom (convex) corner (8.0; 18.31) is rounded with the same small radius 0.25. Because the step is smaller than the radius, it looks like a soft bend.
- The inner corner (8.0; 18.16) is sharp.

## How to do it (`model-current-and-target.png`, `model-step-zoom.png`)
Replace the body line
```python
s += R(0.0, 4.5, 60.0, 18.31, RS or 0.8, BODY)                   # body
```
with two rectangles. Order matters: outlines together, then fills. That hides the inner outlines and leaves one continuous thin outline outside.
```python
bA = R(0.0, 4.5, 60.0, 18.16, RS, BODY)    # body: bottom at 18.16 (at the far end)
bB = R(8.0, 16.0, 60.0, 18.31, RS, BODY)   # bottom strip from x=8.0 to the key end: bottom at 18.31
s += [bB[0], bA[0], bB[1], bA[1]]          # outline B, outline A, fill B, fill A
```
- `R` mirrors x via `m()` itself, so the blue side (mirror) also gets the step at the far end.
- Nothing else needs to change: the seam 16.45–16.95 and the vertical seam at the end are above 18.16.
- Remove `'seamstep'` (the U-groove).
- The 0.15 height can be a parameter (`ST = 0.15` → `18.31 - ST`) if the user wants it bigger.

## Note on the radius
The user wants one small radius, 0.25, on almost all corners. In this solution:
- far bottom corner — 0.25 (corner of rectangle A);
- key-end corner — 0.25 (B);
- the step's bottom corner — 0.25 (B);
- the top corners of B are hidden under A.
