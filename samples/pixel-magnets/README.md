# Pixel Magnets

A pixel-art take on the **Magnets** (sometimes "Magnet Puzzle") logic
puzzle. The board is pre-tiled with 1×2 / 2×1 dominoes — each one a
magnet. Decide which magnets are charged (one `+` end, one `−`)
and which stay neutral so that no two same-charge cells share an edge
and the row + column counts of charges match. A fresh shading-logic
entry alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign across 4×4 → 6×6 boards with one rectangle
  (4×6, 6×4) in between (must be even cells — a domino tiling
  requires `W × H` to be even, so odd-square `n²` like 5×5 is
  impossible by construction).
- Every puzzle is procedurally generated and **verified** to have
  exactly one solution by a domino-by-domino backtracking solver that
  prunes by:
  * orthogonal same-charge adjacency (across magnets).
  * per-row + per-column `+` / `−` count caps.
- The solver tries each magnet in turn at three states (NEUTRAL /
  `+ -` / `- +`); equal-cell-count constraints + adjacency carve down
  the search.
- Live red conflict highlight:
  * Domino consistency — a 1×2 with two `+` cells or two `−` flashes.
  * Adjacent same-charge cells.
  * Row / column over-count (any same-charge cell in that line).
- Tap any cell to cycle **blank → + → − → empty (X) → blank**.
  Mark cells empty (`X`) as your "this cell is definitely neutral"
  note.
- Row / column counts are colour-coded: red for `+`, blue for `−`.
- Score = `999 - seconds` on a clean solve.
- English / 中文 toggle, `localStorage` save with cleared puzzles and
  best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 44 data checks — every level builds with the right `W × H`,
  every solution wins `isSolved`, every clue set has exactly one
  solution, generator output is bit-for-bit deterministic; plus
  adjacent-same-charge and domino-mismatch conflict cases.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4310
```

Then visit `http://127.0.0.1:4310/index.html`.

## Play

- Tap a cell to cycle through **blank → + → − → empty (X) → blank**.
- The thick-bordered 1×2 cells are magnets — each magnet is either
  neutral (both ends blank/empty) or carries `+` on one end and `−`
  on the other.
- Two cells with the same charge can never share an edge.
- Win when every row and column matches its charge count.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — random tiling, random valid state assignment, domino
  -by-domino backtracking uniqueness solver, live violation + win
  helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, magnet borders (thick) + cell borders (thin),
  `+` / `−` / `X` sprites, row/col charge counts, HUD.
- `js/game.js` — screen flow, tap input, timer, save.
