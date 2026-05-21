# Pixel Binairo

A pixel-art **Binairo** (a.k.a. Takuzu / Binary / Unruly) — fill the
grid with 0s and 1s so that no three of the same value sit in a row,
every line is balanced, and no two rows or columns repeat. A fresh
binary-logic puzzle alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign **Spark → Inferno** on 6×6, 8×8 and 10×10 grids.
- Every puzzle is procedurally generated from a seed and **verified
  uniquely solvable**: a full valid grid is built by backtracking,
  then clues are carved away while a forced-move-propagation +
  backtracking solver confirms the remaining clue set still pins
  down exactly one solution.
- The three Binairo rules enforced and explained in play:
  no three-in-a-row (any direction), equal 0s and 1s in every row
  and column, and no duplicate rows or columns.
- Tap a cell to cycle blank → 0 → 1 → blank; fixed clue cells carry
  a white ring and can't be changed.
- Live red conflict highlight for any three-in-a-row run or an
  over-filled row / column.
- Cool-slate 0 tiles vs warm-amber 1 tiles so the board reads at a
  glance.
- Level select with progressive unlocks, per-puzzle best score
  (`999 − seconds`) saved to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 18 generation checks — a valid full grid builds for
  6 / 8 / 10 and passes `fullyValid`; every one of the six levels
  builds and its carved clue set is confirmed to have exactly one
  solution by the propagation solver.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4338
```

Then visit `http://127.0.0.1:4338/index.html`.

## Play

- Tap a cell to cycle blank → 0 → 1 → blank.
- Never put three of the same value next to each other in a row or
  column.
- Each row and column must end with equal 0s and 1s — and no two
  rows (or two columns) may be identical.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — full-grid backtracking generator, the three-rule
  validity checks, a propagation + backtracking uniqueness solver,
  clue-carving, live conflict detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, 0 / 1 tiles with digit glyphs, fixed-clue
  rings, conflict tint, HUD.
- `js/game.js` — screen flow, tap-to-cycle input, timer, save.
