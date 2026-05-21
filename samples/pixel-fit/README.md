# Pixel Fit

A pixel-art **polyomino packing** puzzle. A rectangular frame is cut
into irregular pieces, then the pieces are tipped into a tray and spun
to random rotations. Rotate and drop every piece back so the frame is
filled exactly — no gaps, no overlaps. A fresh tiling-puzzle genre
alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Hatch → Foundry** on growing frames (4×4 → 7×6,
  4 → 9 pieces).
- Every frame is procedurally cut by a round-robin BFS partition into
  connected pieces of 3–9 cells, so a **solution is guaranteed by
  construction** — the cut itself is one valid packing.
- Each piece is spun to a random non-solved rotation, so every level
  needs both the right **placement** and the right **orientation**.
- Tap-to-play controls built for touch: tap a tray piece to pick it
  up, tap it again to rotate it 90°, tap a frame cell to drop it, and
  tap an already-placed piece to lift it back out.
- Move counter and a per-frame best (fewest placements), with level
  select and progressive unlocks, all saved to `localStorage`.
- Chunky bevelled blocks, a distinct colour per piece so the packing
  reads at a glance.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first
  tap controls.
- Verified: 49 checks — four rotations return a piece to its original
  shape; every level's pieces are connected polyominoes whose cells
  sum to exactly the frame area; the generator is deterministic; an
  exact-cover DFS finds a tiling for every level and that tiling,
  replayed through the real `place()` calls, wins; place / pick-up /
  occupancy round-trips behave; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4377
```

Then visit `http://127.0.0.1:4377/index.html`.

## Play

- Tap a piece in the **tray** to pick it up; tap it again to **rotate**
  it 90°.
- Tap a cell in the **frame** to drop the held piece there.
- Tap a piece already in the frame to **lift it back out**.
- Fill every cell of the frame to clear the level.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — BFS frame partition, piece geometry (normalise /
  rotate), placement / pick-up / win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, bevelled frame + blocks, the piece tray, HUD.
- `js/game.js` — screen flow, tap input, save.
