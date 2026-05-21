# Pixel Edgematch

A pixel-art **edge-matching tile puzzle**. The board is fully tiled
with square tiles, each carrying a colour on every edge. The tiles
have been scrambled — swapped around the board and spun. Swap and
rotate them back so every shared edge is the same colour on both
sides. A fresh constraint-puzzle genre alongside the other `samples/`
pixel games.

## Features

- 6-room campaign **Hearth → Cathedral** on growing boards (3×3 →
  6×5) with 3–5 edge colours.
- Every board is built from a **solved colouring** — each internal
  grid edge gets a colour, the outer border gets the neutral grey —
  then the tiles are spun and permuted, so a solution always exists
  by construction.
- The grey border colour only ever sits happily on the outside, so
  matching every internal edge automatically seats the border tiles.
- Two-tap controls: tap a tile to pick it up, tap another to **swap**
  the pair, or tap the held tile again to **rotate** it 90°.
- Live feedback: every mismatched shared edge is marked **red**, and
  the HUD counts matched edges so you can watch the board resolve.
- Move counter and a per-room best (fewest moves), with level select
  and progressive unlocks, saved to `localStorage`.
- Crisp pixel tiles — four colour wedges meeting at the centre.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 53 checks — four rotations return a tile to its edges and
  a CW turn moves the base left edge to the top; every room builds
  with the right tile count, in-range edge colours, is not pre-solved
  and is deterministic; the canonical layout (tile k at cell k,
  rotation 0) wins with every edge matched; swap and rotate behave
  and the move counter tallies; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4409
```

Then visit `http://127.0.0.1:4409/index.html`.

## Play

- Tap a tile to **pick it up** (it highlights).
- Tap a **different** tile to swap the two; tap the **held** tile
  again to rotate it 90°.
- Every edge shared between two tiles must show the same colour —
  red marks the ones that don't yet.
- Match every shared edge to clear the room.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — solved-colouring generation, the scramble, edge
  reading, swap / rotate / win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, four-wedge tiles, red mismatch marks, HUD.
- `js/game.js` — screen flow, tap input, save.
