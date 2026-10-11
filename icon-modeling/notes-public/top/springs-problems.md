# Springs on the end faces (top and bottom): what the model gets wrong

## How I looked
- **Photos:** all 8 end-face photos. Rotate them by EXIF first.
- **Rectification:** each end rectified by its corners into model mm. Width W = 30.4. y as in `bottom_new` / `top_new`.
- **Image** `springs-photo-model.png`. For each end, in rows:
  - photo;
  - model now (`fx=('coil',)`);
  - proposal.
- **The springs sit deep in the niches, so parallax makes them "move" by 1–2 mm from frame to frame.**
  - Positions come from the most frontal frames: bottom — f008428b and a4c95a21, top — 03575498.
  - On the strictly frontal 11c15aa4 the bottom springs are almost invisible. They show only when tilted: the left one on f00, the right one on a4c.

## Answers

### 1. Where the springs are
| end | how many | x, mm | y, mm | touching what |
|---|---|---|---|---|
| bottom (USB-C) | 2: one on each side | **5.1–7.8** and mirrored **W−7.8…W−5.1** (22.6–25.3; a4c shows 24.9–27.8 — parallax) | **2.2–3.15**: lower half of the gap | stand on the body (end-face top 3.36), do not reach the clip (1.43). The left one is tight against the left end of the dark block (7.9) |
| top (jacks) | 2: one in each niche | **4.8–7.6** (4 turns) and **21.0–23.1** (3 turns) | **14.0–15.3**: upper half of the niche | hang from the niche's top edge (body), do not reach the strut on the clip (16.0) |

- **About "top and bottom":** these are the springs on the top end (jacks) and on the bottom end (USB-C). One pair per end, 4 in total.
- The side faces have no springs. The side gap holds tabs (clip tongue + body lug), not springs. Nothing to add on the side.

### 2. What a spring looks like
- The spring lies **across the end face** (axis along x) and is seen from the side.
- Each turn is a **thick short post** ~0.3 mm wide.
- **3–4 turns**, pitch **~0.85–1.0 mm**.
- Height ~1.0 mm (bottom) / ~1.3 mm (top).
- The turns are **dark metal**, each with **one bright glint**: on the top end at the upper end, on the bottom at the upper (outer) end.
- **Background behind the spring:**
  - on the bottom — a black pocket;
  - on the top — a dark niche, and beside the spring a **grey porous pad (foam / felt)** with speckles: x ≈0.5–4.5 on the left and ≈23.6–27.6 on the right.
  - Under the spring, on the clip, lies a **light strut**.

### 3. Other possible readings
- **"Springs at the top and bottom of each niche"** — no. Each niche has one spring, in one height band.
- **"On the side faces at both ends"** — the side photos show two tabs at the far end, not springs. At the key end, in dark niche 19 on photo 7a8f2591, there is roughness ("teeth"), but that is foam/texture, not turns. Nothing to add; if the user meant that, speckles like `felt()` would do.

## Model problems (by importance)
1. **The turns are thin long sticks, and too many.**
   - Model: 7 turns at 0.5 mm pitch, 2.2 mm tall (top, `coil`) and 3–4 sticks at 0.5 pitch (bottom).
   - Photo: 3–4 thick posts at ~0.9 mm pitch, 1.0–1.3 tall.
   - So it reads as a grille / comb, not a spring. **This is the main reason it looked "wrong".**
2. **Glints.** The model has a glint on both ends of each turn (v2) or none at all (v1). The photo has **one bright glint at one end** of a turn; the rest is dark.
3. **Bottom: the springs are in the wrong place.**
   - Model: x 2.9–4.7 (right at the edge).
   - Photo: 5.1–7.8 — tight against the dark block. A ~2.3 mm shift.
   - Height: model 1.7–3.15 (almost the whole gap), photo 2.2–3.15 (standing on the body).
4. **Top: the springs are too tall and in the wrong part of the niche.**
   - Model: 14.0–16.2 (almost the whole niche).
   - Photo: 14.0–15.3 — hanging from the top edge, with an empty dark band and the strut below.
   - x: model 5.2–8.8 / 21.1–24.7; photo 4.8–7.6 / 21.0–23.1. A ~1–1.5 mm shift, and one turn fewer on the right.
5. **No foam in the top niches.** On the photo there is a grey speckled block (≈4 mm wide) next to the spring. In the model the niche is empty and black.
6. **The strut on the top niche floor is too thin.** Model: 0.45 mm (16.43–16.88). Photo: ~0.9 mm (16.0–16.88), light grey.
7. **The top's right niche is probably shorter** — on the photo it ends around x≈27.8, then body. But the angle may be at play (the left side wall is visible). Not sure; not changed in the code.
8. **Other things in the bottom gap** (photos f00 / 11c): the dark block 7.9–20.9 and the rounded lug 21–22.4 match the model within ~0.5 mm. Not changed.

## Code to paste
Functions (next to `coil()`):
```python
def turns(x0, x1, n, y0, y1, glint_top=True):
    """Clip spring lying along x, seen side-on: n thick wire turns, lit only at one end (photos)."""
    s, pitch = [], (x1 - x0 - 0.32) / max(n - 1, 1)
    for k in range(n):
        x = x0 + k * pitch
        s += [rr(x, y0, x + 0.32, y1, 0.16, RING)]                                    # turn, dim metal
        gy = y0 + 0.1 if glint_top else y1 - 0.45
        s += [rr(x + 0.04, gy, x + 0.28, gy + 0.35, 0.12, GLINT)]                    # glint
    return s
def felt(x0, x1, y0, y1, seed=1):
    """Grey speckled foam pad in the niche (photos 035 / 414)."""
    import random; rnd = random.Random(seed)
    s = [rr(x0, y0, x1, y1, 0.1, PLATE)]
    for _ in range(int((x1 - x0) * (y1 - y0) * 3)):
        s.append(circle(rnd.uniform(x0 + 0.1, x1 - 0.1), rnd.uniform(y0 + 0.1, y1 - 0.1), rnd.uniform(0.06, 0.12), STEEL))
    return s
def bottom_springs():          # USB-C end: replaces the 'coil' branch (gap y 1.43 … 3.36, body below)
    return ([rr(W - 8.0, 1.43, W - 4.6, 3.36, 0.1, SHADOW)]                  # dark pocket for the right spring
            + turns(5.1, 7.8, 4, 2.2, 3.15, glint_top=True)                  # f008428b: x 5.1–7.8, standing on the body
            + turns(W - 7.8, W - 5.1, 4, 2.2, 3.15, glint_top=True))         # a4c95a21: mirrored
def top_springs():             # jack end: replaces the 'coil' branch + the strut line (niche y 13.6 … 16.88)
    yc = 16.88
    return (felt(0.5, 4.5, 14.1, 15.9, 1) + felt(23.6, 27.6, 14.1, 15.9, 2)    # foam beside the springs
            + turns(4.8, 7.6, 4, 14.0, 15.3) + turns(21.0, 23.1, 3, 14.0, 15.3)   # 03575498: hang from the niche top
            + [rr(0.6, 16.0, 10.3, yc, 0.1, STRUT), rr(19.6, 16.0, W - 0.6, yc, 0.1, STRUT)])  # strut 0.9 thick
```
- **In `bottom_new`:** instead of the `if 'coil' in fx: ... else: ...` branch (both branches) — `s += bottom_springs()`.
  - Same position: right after `rr(2.0, 1.43, 8.0, 3.36, 0.1, SHADOW)`.
  - The function needs `W` inside: pass it as a parameter or define it inside.
- **In `top_new`:**
  - remove the `if 'coil' in fx: ... else: ...` branch;
  - remove the line `s += R(0.6, yc - 0.45, tx0, yc, 0.1, STRUT) + R(tx1, yc - 0.45, W - 0.6, yc, 0.1, STRUT)`;
  - put `s += top_springs()` in their place (after the dark niches, before the body).
- Existing colours only: RING, GLINT, PLATE, STEEL, STRUT, SHADOW. The turn radius 0.16 is the wire's semicircular ends, not a part corner.
- Checked by rendering: the bottom rows of both blocks in `springs-photo-model.png`.
