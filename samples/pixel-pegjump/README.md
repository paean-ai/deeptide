# Pixel Peg Jump

A pixel-art take on **peg solitaire** (a.k.a. Brainvita / Solo Noble) —
jump a peg over an adjacent peg into an empty hole, removing the jumped
peg. Reduce the board down to a single peg to clear a level. A fresh
single-player logic genre alongside the other `samples/` pixel games.

## Features

- 6-board campaign **Sprout → Cathedral** — 3×4 starter, 4×4, 3×6, 5×4,
  5×5, and the classic English 33-peg cross.
- Every board was **searched** by an in-process DFS solver and is
  verified to reduce to a single peg. Boards with parity obstructions
  (e.g. the 5×5 plus with a central hole) are excluded by construction.
- Tap a peg to select; legal landing holes light up with a green ring.
  Tap the landing hole to jump — orthogonal only, distance 2, over a
  peg, into an empty hole.
- **Undo** every move (unlimited) and **Restart** at any time.
- Star rating per board: 3 = solved to 1 peg, 2 = 2 left, 1 = stuck.
- Level select with progressive unlocks; per-level best star count
  persisted to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 14 mechanics checks — boards build, every level has a
  legal opening move, tap-select-then-tap-target plays a legal jump,
  jumped peg is removed, undo restores state, restart wipes state,
  and the DFS solver confirms each of the six boards reduces to a
  single peg.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4318
```

Then visit `http://127.0.0.1:4318/index.html`.

## Play

- Tap a peg to pick it up; legal landings ring green.
- Tap a green ring to leap — the jumped peg vanishes.
- Reduce the board to **one peg** to win.
- Use **Undo** to back out of dead ends; use **Restart** to wipe the
  board.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — board layouts (verified solvable), tap/jump rules,
  undo stack.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, board card, pixel-disk pegs + sockets,
  selection / landing rings, HUD, star rating.
- `js/game.js` — screen flow, pointer input, save.
