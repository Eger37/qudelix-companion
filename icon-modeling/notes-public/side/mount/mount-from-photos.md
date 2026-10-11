# Clip mount at the key end (x 38–60) — from photos and the drawing

## How to read
- Coordinates in model mm. x: 0 — far end, 60 — key end. y: 0 — clip top.
- **Sources:**
  - photo 7a8f2591 (new, main);
  - photo 6f41cdc1 (close-up, side);
  - photo 4ec62f48 (bright light);
  - photo 145e0bdc (at an angle);
  - drawing `refs/box-side-volume.jpg`.
- **Rule:** shape and tone from the photos, exact positions and sizes from the drawing. The photos have perspective: the pin comes out lower there (y≈3.9–4.0 vs 3.6 on the drawing), so coordinates come from the drawing.

## Images
- `photo-1-new-7a8f2591.png`, `photo-2-6f41cdc1.png`, `photo-3-4ec62f48.png`, `photo-4-145e0bdc.png` — photos with every part labelled.
- `drawing-mount.png` — the same spot on the drawing.
- `model-current-a-b.png` — the model now and the proposed variants.

## How it is built (top to bottom, left to right)
1. **Clip** — the metal strip on top, full length.
2. **Left bracket** (scan 9), x≈40.2–44.8.
   - A narrow strip right under the clip, tight against it. Only a thin dark line between them.
   - Thickness ~0.7 mm (y 1.43–2.1).
   - Left end: straight, short end. It hangs over the see-through gap.
   - Right end wedges down along the ear's left shoulder, to (44.28; 2.43).
   - **Tone: like the clip, metal, lighter than the body.** Light on 6f41cdc1 and 145e0bdc. Dark on the new photo, but exactly the same as the clip itself. So it is part of the clip (its mounting bracket), not the body.
   - On 6f41cdc1, under the part over the gap (x 40.6–42.8), a dark edge of ~0.3–0.5 mm shows — the bracket's thickness in shadow.
3. **Piece 10**, x 42.4–43.4…44.3.
   - Body in shadow: dark, almost black.
   - Runs from the bracket down to the body top.
   - Left edge vertical; the see-through gap ends there. Right edge is the ear's slanted flank.
4. **Ear** — a body boss from the body top up to the clip.
   - Body colour, slightly lighter than the shadow on either side.
   - Left flank a straight slant. Flat top pressed against the clip. Right flank kinked.
   - The shape is already in the model (v46).
5. **Axle/pin** in the ear, centre (46.5; 3.6) per the drawing.
   - On all photos it is a **metal ring**: light, Ø≈1.6 mm (r≈0.8). Inside a **dark hex socket**, r≈0.35.
   - Around the ring a slightly lighter body spot, r≈1.1–1.2.
   - Same on the drawing: a dark circle with a light middle.
6. **Right bracket** (scan 18), x≈47.9–52.9.
   - Same as the left one, mirrored: tight under the clip, ~0.6 mm thick.
   - Left end wedges down along the ear's right shoulder, to (48.9; 2.48). Right end straight, x 52.9.
   - **Tone like the clip and the left bracket** (light on 6f41cdc1, 4ec62f48, 145e0bdc).
7. **Dark niche** (scan 19), from the ear to the end.
   - Body in shadow, almost black.
   - Under the right bracket it spans y 2.05–4.5, and from x 52.9 to the end — from the clip itself (1.43) to the body top.
   - On the new photo there is roughness inside ("teeth"), and a light edge line along its lower border.
8. **End wall**, x≈59.65–60.
   - A thin body wall closes the niche on the right and runs from the body top to the clip.
   - On the photo — a light highlight along the edge. On the drawing — one thick end line.
   - **No cut or gap at the clip end** (see the clip report).

## What the model gets wrong
- **Main: both brackets are BLACK now** (JACK) — plain outlined rectangles that merge with the shadow.
  - On all photos they are **metal, clip-toned** — part of the clip.
  - Because of this the mount reads as "black mush". In reality it is the clip, two light tabs, a dark ear with a light ring, and a dark niche.
- **The brackets have no wedges** at the ear shoulders. On the drawing and on 6f41cdc1 the brackets' inner ends go down along the ear shoulders.
- **Pin:** in the model a dark disc with a black centre. On the photos and the drawing — a light metal ring with a dark middle. The user earlier decided "no grey ring" — let them look at variant B.
- **The niche and piece 10** already have the right shape: dark, no outline against the body.

## What to do (variant A)
Replace the two bracket lines:
```python
STRUT_C = STRUT          # (0x60,0x60,0x66) — lighter than the clip, as on 6f41cdc1; or CLIP_SCAN (72,72,78) to match the clip exactly
S9  = [(40.2,1.43),(44.83,1.43),(44.78,1.71),(44.56,2.15),(44.28,2.43),(44.06,2.37),(43.73,2.1),
       (40.42,2.1),(40.26,1.99),(40.2,1.82)]
S18 = [(47.9,1.43),(52.9,1.43),(52.9,1.75),(52.7,2.0),(49.45,2.05),(48.9,2.48),(48.4,2.05),(48.0,1.65)]
for pts in (S9, S18):
    s += [P(pts, STRUT_C)]
    for a, b in zip(pts, pts[1:] + pts[:1]):          # thin outline, except along the clip
        if a[1] > 1.44 or b[1] > 1.44:
            s += [("line", m(a[0]), a[1], m(b[0]), b[1], 2 * o, OUTLINE)]
```
- Points are taken from the drawing's line centres.
- Wedges S9 and S18 sit tight against the ear flanks (v46): (44.83; 1.65)–(44.35; 2.5) and (47.95; 1.65)–(48.9; 2.6).
- Draw after the ear and before the clip, as now.

**Variant B** — the same, plus the pin as on the photos:
```python
s += [circle(m(46.5), 3.6, 0.8, (0x8A,0x8A,0x90)),     # metal ring
      circle(m(46.5), 3.6, 0.62, (0x5A,0x5A,0x60)),    # chamfer/shadow inside the ring
      circle(m(46.5), 3.6, 0.35, HOLE)]                # hex socket (as a circle)
```
Centre (46.5; 3.6) is a fit to the drawing. The model has (46.55; 3.5) now, 0.1 mm off.

**Optional:**
- dark edge under the left bracket over the gap: `line` from (40.3; 2.2) to (42.4; 2.2), width 0.15, colour SHADOW;
- end wall: BODY, x 59.65–60.0, y 1.43–4.5.

## Where photos and drawing disagree
| detail | photo | drawing | I take |
|---|---|---|---|
| bracket tone | metal, clip-toned | line art, no tone | photo |
| pin | metal ring + dark hex | dark circle, light middle | photo (variant B), the user decides |
| pin height | y≈3.9–4.0 | 3.6 | drawing (perspective on the photo) |
| bracket thickness | ~0.9–1.0 | 0.6–0.7 | drawing |
| left end of the left bracket | x≈40.6 | 40.2 | drawing |

## In passing (outside this spot)
The posts in the see-through gap at the far end are the other way round on the photos compared with the model:
- x≈7–8 — metal, from the clip;
- x≈11.7–12.8 — black, from the body.

More in `../clip/clip-from-photos.md`.
