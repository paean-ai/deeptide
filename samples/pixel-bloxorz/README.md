# Pixel Bloxorz

A pixel-art take on **Bloxorz** — roll a 1×1×2 block across a tile grid
and land it standing in the goal hole. Roll off the edge and the block
falls. Stand on a weak tile and it falls through. Nine hand-designed
boards, each verified by an in-process BFS solver so the listed par is
the proven optimum. A fresh rolling-block logic genre alongside the
other `samples/` pixel games.

## Features

- 9-level campaign **Sprout → Keystone** with BFS-derived pars of
  **2 / 4 / 6 / 7 / 9 / 10 / 13 / 16 / 19** moves. Each level was
  searched over (col, row, orient) states; the listed par equals the
  minimum-move optimum, so a 3-star clear means you matched the BFS
  solution. The closing trio — **Switchback, Catacomb, Keystone** —
  are compact boards whose tight tracks force long winding routes.
- Three faithful block orientations — **standing** (1-cell footprint),
  **lying horizontally** (2-cell), **lying vertically** (2-cell) —
  with all twelve roll transitions implemented exactly.
- Two tile types: **solid** (supports both lying and standing) and
  **weak** (supports lying — but the concentrated weight of a
  standing block crashes through). Void cells off the carved track
  drop the block.
- **Undo** every move (unlimited stack) and **Restart** at any time.
- Star rating per level: 3 = matched par, 2 = within +50 % of par,
  1 = solved-but-loose. Per-level best (lowest) move count and
  cleared flag saved to `localStorage`.
- Tap the edges of the screen (around a 32 px dead zone) to roll, or
  use arrow keys / WASD on desktop. **Z** undoes; **R** restarts.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 53 checks — all 9 levels are BFS-solvable; the listed par
  equals the BFS minimum; the optimal path replayed through the real
  `tryMove` wins (not fallen) in exactly par moves and scores 3
  stars; the closing levels parse and start on a stand-supporting
  cell; all four scripts load cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4324
```

Then visit `http://127.0.0.1:4324/index.html`.

## Play

- Tap above / below / left / right of the block (or use arrow keys /
  WASD) to roll one step in that direction.
- Cross weak tiles **lying down**. Stand only on solid tiles. Roll
  the block to be **standing on the goal hole** to clear the level.
- Undo a mis-step with **Undo** or **Z**; reset with **Restart** or
  **R**.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — block / tile model, roll transitions, support rules,
  level layouts (BFS-verified solvable at the listed par).
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, bevelled tiles (solid vs weak with cracks),
  goal hole, block (tall standing vs squat lying), HUD, stars.
- `js/game.js` — screen flow, edge-tap + keyboard input, save.
