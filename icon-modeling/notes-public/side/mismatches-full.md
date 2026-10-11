# Box scan vs side model v31 — full list of mismatches

**What was compared.** The scan `box-side-volume-colored-v7.png` (kept outside the project, in the Desktop scan folder) and the line mask `icon-modeling/out/side-volume.npy` converted to model coordinates. The model is `side_ideal()` in `tools/icon_parts.py`, render v31.

**Coordinates** in mm: x = 0 at the far end … 60 at the key end; y = 0 at the clip's outer edge … 18.3 at the bottom. For the scan the tables give the centre of the printed line: the scan's lines are 0.3–0.6 mm thick, so their middles must be compared.

**Icon scale.** If the side is ~900 px wide (1024 icon), 1 mm ≈ 15 px. At 256 px, 1 mm ≈ 4 px. At 32–48 px, 1 mm is less than 1 px.

Severity:
- **★★★** visible on a large icon, changes the part's characteristic shape;
- **★★** visible in a close-up comparison;
- **★** barely visible, optional.

---

## 1. The ear and the shadow round it (key end, x 42–50, y 1.4–4.7)

### 1.1 The ear's left flank is slanted, but vertical in the model — ★★★
Left boundary of the ear (body vs black piece 10):

| y, mm | scan, x | model, x | difference |
|---|---|---|---|
| 1.6 | 44.83 | ≈45.9 | 1.0 |
| 2.0 | 44.67 | ≈45.1 | 0.4 |
| 2.4 | 44.34 | ≈44.7 | 0.35 |
| 3.0 | 44.12 | 44.5 | 0.4 |
| 3.6 | 43.84 | 44.5 | 0.65 |
| 4.2 | 43.51 | 44.5 | 1.0 |

- **Scan:** the left flank is an almost straight slant from (44.85; 1.6) to (43.5; 4.3). The ear widens downward, like a trapezoid.
- **Model:** vertical x=44.5 from y=4.7 to 3.0, rounding inward above that. At the bottom the ear is ~1 mm narrower; at the top the left shoulder is cut off.
- **The right flank matches** within 0.05 mm: on the scan 48.9 at y=2.6, 49.3 at 3.6, 49.5 at 4.2.
- **Fix:** replace the left half of the ear polygon, e.g. `(43.45,4.7),(43.85,3.6),(44.15,3.0),(44.4,2.3),(44.75,1.7),(45.1,1.5)`. Keep the right half as is.

### 1.2 The ear top is flat and touches the clip over ~3 mm, not at one point — ★★★
- **Scan:** the ear body reaches the clip's lower line (boundary y≈1.32–1.49) along all of x≈44.9…48.0. At x=48 strut 18 already starts above the ear (ear up to y 1.65).
- **Model:** the arch `(44.8,2.2),(45.5,1.7),(46.6,1.5),(47.6,1.6),(48.4,1.95)` touches the clip only near x=46.6. At x=45 the ear top is at y≈1.95, i.e. ~0.45 mm lower. At x=48 it is at ≈1.77, ~0.1 mm lower.
- **Fix:** make the top flat, `(45.1,1.5)…(47.7,1.5)`, with rounded shoulders: `(44.75,1.7)` on the left and `(48.1,1.65)` on the right.

### 1.3 Black piece 10 is a trapezoid, not a rectangle — ★★ (fixed together with 1.1)
- **Scan:** left edge x≈42.4. The right edge is slanted and parallel to the ear flank: 43.84 at y 2.2 and 43.51 at y 4.2. Width ~1.9 mm at the top and ~1.15 mm at the bottom.
- **Model:** black zone 42.4…44.5 over the full height, 2.1 mm wide.
- **Fix:** nothing separate is needed. The new ear polygon from 1.1 covers the extra black.

### 1.4 The shadow (piece 10 and the black block at the key) goes below the body's top line — ★
- **Scan:** the shadow's lower boundary y≈4.30–4.36 — level with the see-through gap's lower line (4.36).
- **Model:** shadow down to y=4.62, while the body top under the gap is at 4.5. The shadow is ~0.12 mm lower.
- **Fix:** move the bottom of the shadow polygons from 4.62 to 4.5.

### 1.5 Corner of the black block at the clip end (x 59.6–60) — ★
- **Scan:** black block 19 rises along the end up to y≈1.0–1.15, next to the clip end.
- **Model:** the block and the end outline start at y=1.43. The background shows in the corner between the rounded clip end and the end face. Difference ~0.3 mm.
- **Fix:** raise the block top at the end (x 59.6–60) to ~1.05.

### 1.6 Hole (pin) — ★ (barely visible)
- **Centre:** a circle fit on the scan gives (46.4–46.5; 3.56–3.66), the model has (46.55; 3.5). Offset ≤0.15 mm.
- **Radius** matches: line centre ~1.1, the model 1.15.
- **The centre on the scan is uneven:** the black dot is shifted ~0.3 mm toward the far end, and a short "tail" (~0.3 mm) goes up from it. The model has a clean r=0.5 dot in the centre; that is intended, an idealisation.

## 2. Struts under the clip

### 2.1 Left strut 9 is shorter than on the scan, and the hook is bigger — ★★
- **Strut start:** on the scan the outer edge of its end is at x≈39.8, the line centre at 40.2. In the model the fill starts at 40.4, the outline at 40.31. The model is ~0.3–0.5 mm shorter on the far-end side.
- **Hook:** on the scan the strut's left end is just a thick end, its bottom only ~0.1–0.2 mm below the main strut (down to y≈2.2). In the model a 0.6 mm wide hook goes down to 2.25 (2.34 with the outline), i.e. 0.3 mm below the strut's bottom.
- **Thickness:** on the scan the strut bottom is at y≈2.10–2.15 (line centre), in the model 1.96. The model is ~0.15 mm thinner.
- **Fix:** `R(40.0, 1.43, 44.5, 2.1, …)`, reduce the hook to y≈2.2 or remove it.

### 2.2 Right strut 18 — matches (x 48.3–52.9, bottom ~2.05–2.15)

### 2.3 Spring posts in the gap (5 and 7) — match
- Post 5: on the scan x 6.7–7.94, in the model 6.7–8.0.
- Post 7: on the scan 11.08–12.35, in the model 11.0–12.4.
- On the scan both posts are capsules with rounded ends; in the model rectangles rounded 0.15 at the bottom only. ★, barely visible.

## 3. Far end (x 0–5)

### 3.1 Right side of the step toward the clip — ★★
- **Scan:** the step top is at y=3.36, same in the model. The slope starts earlier and is gentler: at x=3.5 the top is 3.42, at x=4.0 already 4.08, at x=4.5 4.63. The inner corner at the base has a concave fillet of radius ~0.3 mm.
- **Model:** rounding 0.6 and a vertical wall at x=4.1 down to y=4.5. Sharp inner corner. At x=4.0 the model top is 3.63, the scan 4.08: ~0.45 mm height difference. The model's outer corner is ~0.2–0.3 mm "fuller" overall.
- **Fix:** move the wall to x≈3.9 and tilt it slightly, e.g. to 4.2 at the base. In the inner corner (4.1–4.4; 4.2–4.5) add a concave fillet r≈0.3, i.e. a body fill with a circle cut out.

### 3.2 The end is slightly tilted — ★ (most likely the drawing's perspective)
- **Scan:** the end is at x=0.00 at y=4.0, 0.17 at y 5–6, 0.22 at y=8 and 0.28 at y ≥10. The top sticks out by ~0.28 mm.
- **Model:** vertical x=0.
- **Advice:** leave it. This falls under the decision "straight lines instead of scan wobble".

### 3.3 Far-end bottom corner — ★
- **Scan:** the corner is almost square, radius ~0.3 mm. At y=17.6 the end is still at x=0.28, same as above.
- **Model:** radius 0.8, ~0.9 with the outline. The corner is noticeably rounder.
- **For comparison:** the bottom corner at the key end has radius ~0.7 on the scan and matches the model.
- **Fix:** radius 0.3 at the far bottom corner only. E.g. cover the corner with a rectangle `rr(0, 17.5, 1, 18.31, 0)` over the body and round it separately.

### 3.4 Notch at the seam — ★ (artefact)
On the scan, where the seam line meets the end (y≈16.4–16.6), the outline is pushed in by ~0.15 mm. That is the join of two rounded regions with a thick line. Do not copy it.

## 4. Key (x 42.4–60, y 8–11.9)

### 4.1 Corner radius — ★★
- **Position and size match:** on the scan line centre x 42.41…~60, y 8.1–11.9. In the model 42.4–60, 8.0–11.9; the top differs by 0.1 mm.
- **The rounding differs:** from the line's 0.1 mm offset from the edge, the scan's corner radius is ≈0.5 mm. The model has 0.7, ~0.8 with the outline.
- The key is the most contrasting part, so rounder corners show on a large icon.
- **Fix:** `R(42.4, 8.0, 60.0, 11.9, 0.5, key)`. The highlight, if kept, with r≈0.35.

## 5. Clip

### 5.1 Clip ends — ★
On the scan the ends are almost rectangular, radius ~0.2–0.3 mm. In the model 0.45. The end positions match:
- at the far end, scan x≈0.5, model 0.5;
- at the key, scan ~59.6–59.75, model 59.7.

Thickness matches: ~1.1–1.16 mm.

## 6. Overall side geometry

### 6.1 Skew and taper (the scan is not rectangular) — ★ (most likely scan distortion or perspective)
Line centres on the scan:

| x | clip top | clip bottom | body top | seam | bottom |
|---|---|---|---|---|---|
| 1–9 | 0.55–0.61 | 1.65–1.76 | 4.58 | 16.54–16.60 | 17.81–18.03 |
| 30 | 0.28 | 1.49 | 4.36 | 16.65 | 18.09 |
| 59 | 0.00 | 1.16 | 4.30 | 16.76 | 18.25 |
| **model** | 0.33 | 1.43 | 4.5 | 16.70 | 18.31 (+0.09 outline) |

- On the scan the clip and the body top drop toward the key end by 0.3–0.6 mm, and the bottom goes down by ~0.45 mm.
- Body height on the scan ~13.45 mm at the far end and ~13.95 at the key. In the model constant, 13.8.
- Because of this the bottom strip under the seam is 1.27 → 1.49 mm on the scan, ~1.6 in the model. At the far end the model is ~0.4–0.5 mm lower than the scan.
- The gap between clip and body is 2.9–3.0 on the scan, 3.07 in the model; matches.
- **Advice:** don't copy it, it is print or scan curvature. If you want it closer, raise the bottom evenly from 18.31 to ~18.2: the average error drops.

### 6.2 Notches on the key end — ★ (artefact)
On the scan the key end (x≈59.8–60) is pushed in by ~0.1–0.2 mm where inner lines meet it: at y≈4.3, 8.0, 11.8–12.5 and 16.6. These are thick-line joins; do not copy.

---

## Matches (checked, difference ≤0.1–0.15 mm)
- Height and width of the far-end step: y 3.36, x 0–4.1.
- Spring posts 5 and 7, right strut 18.
- The ear's right flank.
- Hole position and radius.
- Key position.
- Clip and gap thickness.
- Seam position (centre 16.5–16.76, model 16.7).
- Key end flush with the end face.
- Right edge of piece 10 and of the see-through gap (x≈42.4).

## Deliberate differences (the user's decisions)
Not errors; listed for completeness.
- **Colours:** regions 5, 10, 19, 9 and 18 black; post 7 and the clip grey; the seam band black.
- **Key in the "lit" look:** red frame and light centre. On the scan the key is flat red.
- **Far end:** vertical seam along the edge, a small wedge at the top and a thin rim on the rounded corner.
- **Recess in the far-end edge** (variant D, y 5.0–5.7). The scan has no such cut.
- **Outlines:** thin, 0.09 mm, instead of the scan's thick printed lines (0.3–0.6 mm). No outline between the shadow (10 and 19) and the body/ear, only a change of tone.
- **Idealisation:** straight lines and true circles instead of scan wobble. Items 3.2, 3.4, 6.1 and 6.2 belong here, and partly the shape of the hole centre (item 1.6).

## Fix order by value
1. Ear: left flank and flat top (1.1 and 1.2). Piece 10 then fixes itself (1.3).
2. Right side of the far-end step (3.1).
3. Left strut 9: length, thickness and hook (2.1).
4. Key corner radius 0.7 → 0.5 (4.1).
5. Minor: shadow bottom 4.62 → 4.5 (1.4); block corner at the clip end (1.5); clip-end and far-bottom-corner radii (5.1, 3.3).
