# Pixel Kakurasu

A pixel-art take on **Kakurasu** — the index-sum logic puzzle. Each
row's "score" is the **sum of column indices** of the shaded cells in
that row; each column's score is the sum of row indices of shaded
cells in that column. Match every row and column target. Distinct
from Sumplete (index sums vs raw value sums) and from the cage-
arithmetic puzzles in `samples/` (kenken, kakuro).

## Features

- 9-puzzle campaign on 4×4 → 8×8 grids (**Brook → Abyss**).
- Every puzzle is procedurally generated and **verified** to have
  exactly one solution by a row-by-row subset solver that enumerates
  every subset of each row whose column-index sum equals the row
  target and prunes by column over-sum.
- Index labels printed above each column (1..n) and beside each row
  so you can read off scoring at a glance.
- Live red conflict highlight:
  * Row whose SHADED row-sum already exceeds the target, *or* whose
    SHADED+UNDECIDED row-sum can no longer reach the target.
  * Same rule applied per column.
- Tap any cell to cycle **undecided → shaded → empty → undecided**.
- Score = `999 - seconds` on a clean solve.
- English / 中文, `localStorage` save with cleared puzzles + best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 31 data checks — every level builds, every solution wins
  `isSolved`, every clue set has exactly one solution, generator
  output is bit-for-bit deterministic; plus a row-overshoot conflict
  case.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4316
```

Then visit `http://127.0.0.1:4316/index.html`.

## Play

- Tap any cell to cycle through **undecided → shaded → empty → undecided**.
- For each row, the **yellow** target on the right = the sum of the
  column indices (1..n) of the shaded cells in that row.
- For each column, the **green** target below = the sum of the row
  indices (1..n) of the shaded cells in that column.
- Win when every cell is decided and every row + column target is hit.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — random-shading generator, row-by-row subset uniqueness
  solver with column-cap pruning, live violation + win helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, index labels (1..n), shaded / empty cell tints,
  row/col target chips colour-coded by axis, HUD.
- `js/game.js` — screen flow, tap-to-cycle input, timer, save.
