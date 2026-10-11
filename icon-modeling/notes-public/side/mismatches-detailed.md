# Scan vs model v36 — the 6 main mismatches in detail

## How to read
- Coordinates in model mm. x = 0 at the far end, 60 at the key end. y = 0 at the clip's outer edge, growing downward.
- **Scan points are the centre of the printed line**: the midpoint between two regions, from the region labels. The scan's lines are 0.3–0.6 mm thick.
- Each image has three panels:
  - **SCAN** — the coloured scan, line centres in cyan;
  - **MODEL** — the current model (`side_ideal`, v36);
  - **MODEL + scan** — the model with the scan's line centres in cyan, the scan contour points from the lists below in yellow, and the current model points as magenta circles.
- Grid: every 0.5 mm, labels every 1 mm.
- The cyan vertical past the end (x≈60.5) is the edge of the scan image, not a part.
- Correction to the earlier report: after precise tracing, items 3, 4 and 5 turned out smaller than they seemed. The numbers below are exact.

---

## 1. Ear, left flank, and black piece 10 — `1-ear-left-flank.png`

**On the scan.**
- The ear's left flank is an almost straight slanted line, no kinks.
- It runs from the ear's top-left corner (44.83; 1.6) down and left to (43.35; 4.36), where it meets the body top. About 28° from vertical.
- The ear is wider at the bottom than at the top.
- To the right of this line is piece 10. Its right edge is the same slant. So piece 10 is a trapezoid: ~1.9 mm at the top, ~0.9 mm at the bottom.
- The left edge of piece 10 is almost vertical, x≈42.4–42.5.

**In the model.**
- The ear's left flank is vertical, x=44.5 from y=4.7 to 3.0. Above that it curves right to (44.8; 2.2) and (45.5; 1.7).
- Piece 10 is a rectangle 42.4–44.2, and the black goes to 44.5.

**Difference.**
- At the bottom (y≈4.3) the model's ear is ~1.1 mm narrower.
- In the middle (y 3.0) ~0.4 mm narrower.
- At the top (y 1.7) the model's left shoulder is cut off: on the scan it is at x≈44.83, in the model ≈45.5.

**Scan contour, ear's left flank (top to bottom):**
```
(44.83,1.54),(44.83,1.71),(44.61,2.15),(44.28,2.48),(44.34,2.76),(44.06,3.03),(43.90,3.36),(43.84,3.64),(43.68,3.97),(43.35,4.36)
```
**Piece 10 on the scan (line centres, closed):**
```
(42.52,4.30),(43.35,4.30),(43.62,3.97),(43.79,3.64),(43.84,3.36),(44.01,3.03),(44.28,2.76),(44.23,2.48),(44.06,2.43),(43.73,2.15),(42.85,2.15),(42.57,2.26),(42.46,3.09),(42.35,3.92),(42.35,4.19)
```
**Model now (ear):**
```
(44.5,4.7),(44.5,3.0),(44.8,2.2),(45.5,1.7),(46.6,1.5),(47.6,1.6),(48.4,1.95),(48.9,2.6),(49.35,3.6),(49.7,4.7)
```
**What to do.**
- Replace the left half of the ear polygon with a straight slant: `(43.35,4.7),(43.7,3.9),(43.9,3.36),(44.1,3.0),(44.35,2.5),(44.65,2.1),(44.83,1.65)`.
- Piece 10 needs no separate change: the ear is drawn on top and covers the extra black. `rr(42.4,2.1,44.2,4.62)` can stay.

---

## 2. Ear top — flat and against the clip — `2-ear-top.png`

**On the scan.**
- The ear top is an almost straight horizontal at the level of the clip's lower line (y≈1.32–1.43), from x≈45.0 to ≈47.9. That is about 3 mm.
- There is no gap between the ear and the clip.
- The corners have short roundings:
  - left — from (45.28; 1.38) to (44.83; 1.6);
  - right — from (47.87; 1.49) to (47.98; 1.71).
- The right flank first goes steeply down-right at ~45°: from (47.98; 1.71) to (48.86; 2.54).
- Then steeper, ~15–20° from vertical: to (49.47; 4.19).
- At the very bottom a small outward rounding into the body top: (49.85; 4.36).
- The ear is asymmetric: the left flank is one straight line, the right one has a kink at y≈2.5.

**In the model.**
- The top is an arch touching the clip near x=46.6 (y 1.5).
- At x=45.5 the top is at y=1.7, at x=45 at ≈1.95, at x=48 at ≈1.77.
- The model's right flank matches the scan within 0.05 mm down to y≈3.7. At the very bottom (y 4.3) the model is ~0.25 mm narrower.

**Scan contour, top and right flank (left to right, then down):**
```
(44.83,1.54),(45.28,1.38),(45.77,1.32),(46.60,1.32),(47.10,1.38),(47.48,1.43),(47.87,1.49),(47.98,1.71),(48.86,2.54),(48.97,3.03),(49.19,3.42),(49.25,3.69),(49.47,4.19),(49.85,4.36)
```
**What to do.** Together with item 1 — a whole new ear polygon. Top at 1.43 (the model's clip bottom), base at 4.7:
```
(43.35,4.7),(43.7,3.9),(43.9,3.36),(44.1,3.0),(44.35,2.5),(44.65,2.1),(44.83,1.65),(45.2,1.43),(47.6,1.43),(47.95,1.65),(48.4,2.1),(48.9,2.6),(49.05,3.1),(49.25,3.6),(49.5,4.2),(49.85,4.7)
```
The hole C(46.55; 3.5) stays inside: ≥0.4 mm to the left flank.

---

## 3. Far-end step, right side — `3-step.png`

**On the scan.**
- The step top is straight, y=3.36, up to x≈3.3.
- Then a smooth arc down (radius ~0.7) to (4.0; 4.05).
- Then a short vertical at x≈4.03, from y 4.05 to 4.25.
- At the bottom a **concave fillet** (radius ~0.35) into the body top: (4.14; 4.47), (4.25; 4.58), (4.36; 4.63).
- The body top here is y≈4.6.

**In the model.**
- An arc of radius 0.6 from (3.5; 3.36) to (4.1; 3.96).
- Then a vertical x=4.1 down to y=4.5. Sharp inner corner, 90°.

**The difference is small.**
- The model's wall is ~0.07 mm further out, ~0.15 with the outline.
- The model's arc is slightly tighter: 0.6 vs ~0.7.
- The main thing — no concave fillet at the base: a sharp corner vs an arc of ~0.35 mm.

**Scan contour (left to right):**
```
(0.72,3.36),(3.31,3.36),(3.53,3.42),(3.75,3.53),(3.92,3.69),(3.97,4.03),(4.03,4.08),(4.03,4.25),(4.14,4.47),(4.25,4.58),(4.36,4.63),(4.91,4.63)
```
**Model now:** `R(0.0, 3.36, 4.1, 5.0, 0.6, BODY)`, wall x=4.1, corner (4.1; 4.5) sharp.

**What to do.**
- Outer radius 0.6 → 0.7 (optional).
- The wall can stay at 4.1.
- Add a BODY fill with a concave arc in the inner corner: `poly (4.1,4.15),(4.12,4.27),(4.2,4.4),(4.33,4.48),(4.45,4.5),(4.1,4.5)` (arc centred at (4.45; 4.15), r 0.35).
- A thin outline along the arc. Hide the wall outline below y 4.15.

---

## 4. Left strut 9 under the clip — `4-strut-9.png`

**On the scan.**
- The strut is a straight band from x≈40.2 to the ear. Top is the clip line (y≈1.32), bottom straight, y≈2.04–2.10.
- **Left end** — just a rounded end: (40.2; 1.49) → (40.2; 1.82) → (40.42; 2.10). Radius ~0.25.
- **There is no downward hook at the left end.** The bottom is level along the whole length.
- **Right end** — a wedge going down along the ear's left shoulder: (43.73; 2.10) → (44.28; 2.43) → (44.56; 2.15) → (44.83; 1.49). It is black, and the model is also black there (shadow), so it looks almost the same.
- Right strut 18 is built the same way, mirrored: its left end also wedges down along the ear shoulder, to (48.9; 2.48).

**In the model.**
- Strut `R(40.4,1.43,44.5,1.96)`. With the outline the bottom is at 2.05 — matches the scan (2.07).
- Hook `R(40.4,1.43,41.0,2.25)`, down to 2.34 with the outline. It sticks out ~0.25–0.3 mm below the scan over x 40.3–41.1.
- Left end at 40.4, 40.31 with the outline. The scan has 40.2, a difference of ~0.1–0.2 mm.

**Scan contour (closed):**
```
(40.20,1.49),(40.20,1.82),(40.26,1.99),(40.42,2.10),(41.08,2.10),(42.13,2.04),(43.73,2.10),(44.06,2.37),(44.28,2.43),(44.56,2.15),(44.78,1.71),(44.83,1.49),(44.45,1.32),(40.48,1.32)
```
**What to do.**
- Remove the hook: the line `R(40.4, 1.43, 41.0, 2.25, 0.2, JACK)`.
- Move the strut start to 40.25: `R(40.25, 1.43, 44.5, 1.96, 0.25, JACK)`.
- Keep the thickness.

---

## 5. Key corners — `5a-key-left-edge.png`, `5b-key-end.png`

**On the scan.**
- The key is a rectangle with small roundings, radius ~0.35–0.45 mm:
  - top left: (42.46; 8.44) → (42.63; 8.22) → (42.85; 8.16), r≈0.35;
  - bottom left: (42.35; 11.42) → (42.46; 11.64) → (42.68; 11.86) → (42.96; 11.91), r≈0.45;
  - top at the end: (59.61; 8.05) → (59.72; 8.16) → (59.89; 8.33) → (59.89; 8.49), r≈0.3–0.4;
  - bottom at the end: (59.61; 11.86) → (59.78; 11.69) → (59.83; 11.47) → (59.89; 11.42), r≈0.35–0.45.
- Sides (line centre): left x 42.41–42.46, right (end) 59.89–59.94, top y 8.05–8.16, bottom 11.80–11.91.

**In the model.** `R(42.4, 8.0, 60.0, 11.9, 0.7, key)`: radius 0.7, 0.79 with the outline. Sizes match (±0.1).

**Difference.** The model's corners are rounder. The rounding takes 0.7–0.8 mm vs ~0.4. The key is the brightest part, so it shows.

**What to do.** `R(42.4, 8.0, 60.0, 11.9, 0.4, key)`. Light centre r≈0.3 (now 0.5).

---

## 6. Minor items

### 6a. Shadow bottom — `6a-shadow-bottom.png`
- **Scan:** bottom of piece 10 at y=4.30, bottom of the black block right of the ear at 4.25–4.36. That is level with the body top (where the see-through gap's bottom is too, ~4.3–4.36).
- **Model:** shadow down to 4.62, while the body top under the gap is at 4.5. The shadow is 0.12 mm below the body line.
- **What to do:** in the two shadow polygons and in `rr(42.4,2.1,44.2,4.62)` replace 4.62 with 4.5.

### 6b. Clip end and the black block at the key end — `6b-clip-end-key-side.png`
- **Scan, clip:** the end is almost straight, x=59.72 from y 0.33 to 0.83. Short corners, r≈0.25–0.3:
  ```
  (59.17,0.0),(59.45,0.06),(59.67,0.22),(59.72,0.33),(59.72,0.83),(59.67,0.94),(59.56,0.99),(59.28,1.05)
  ```
- **Scan, block 19:** the black block at the end rises up toward the clip end. The cut is slanted: from (59.61; 0.99) to (59.94; 1.43), then the end goes down.
  ```
  (59.23,1.16),(59.61,0.99),(59.72,0.99),(59.83,1.21),(59.94,1.43),(59.94,3.31)
  ```
- **Model:** clip with radius 0.45 (almost a semicircle). The block and the end outline only start at y=1.43. Background shows under the clip's rounding — a triangle roughly (59.3–60; 1.0–1.43).
- **What to do:**
  - clip radius 0.45 → 0.25;
  - add a JACK polygon `(59.3,1.43),(59.6,1.0),(59.75,1.0),(60.0,1.4),(60.0,1.43)` (the clip is drawn later and goes on top);
  - raise the end outline from 1.43 to ~1.1.
- Also, at this end the scan's clip is ~0.3 mm higher than the model. That is the scan's overall skew; do not copy.

### 6c. Clip end at the far end — `6c-clip-end-far.png`
- **Scan:** the end x=0.50 from y 0.83 to 1.54. Top corner r≈0.25, bottom almost square (~0.1):
  ```
  (0.99,0.61),(0.72,0.66),(0.61,0.72),(0.50,0.83),(0.50,1.54),(0.55,1.65)
  ```
- **Model:** x=0.5 (matches), radius 0.45.
- **What to do:** radius 0.25 (shared by both ends, `R(0.5, 0.33, 59.7, 1.43, 0.25, CLIP_SCAN)`).

### 6d. Far-end bottom corner — `6d-bottom-corner-far.png`
- **Scan:** the end below the seam is almost vertical (x≈0.17–0.2) down to y≈17.45. Then a short rounding r≈0.3 into the bottom: (0.28; 17.65), (0.39; 17.76).
  ```
  (0.17,16.88),(0.17,17.43),(0.28,17.65),(0.39,17.76),(2.04,17.76)
  ```
- **Model:** radius 0.8, ~0.9 with the outline. The model's corner is big and round. The bottom corner at the key is ~0.7 on the scan, and the model matches there.
- On the scan the bottom at the far end is at 17.76, in the model 18.31. That is the scan's overall skew; do not copy. Compare only the radius.
- **What to do:** far bottom corner radius ~0.3, keep the key-end corner at 0.8.
  - E.g. cover the corner with a BODY rectangle with r 0.3: x 0…1.0, y 17.0…18.31.
  - Add the outline only along the bottom and the end.
  - Hide the top and right edges of that rectangle under the body fill.

---

## Order
1. Ear (items 1 and 2) — one new polygon.
2. Key corners, r 0.4 (item 5).
3. Remove the strut-9 hook (item 4).
4. Shadow bottom at 4.5 (6a).
5. Clip ends r 0.25, and the block at the clip end (6b, 6c).
6. Concave fillet at the step (item 3) and the far bottom corner (6d).
