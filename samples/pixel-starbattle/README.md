# Pixel Star Battle

A pixel-art take on **Star Battle** — a region-partition + star-placement
logic puzzle. The N×N grid is divided into N irregular regions and you must
place exactly **K stars** in every row, every column, and every region,
with the extra rule that two stars cannot touch each other even diagonally.
A fresh placement-logic puzzle alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign across 5×5 → 8×8 grids (Sky → Quasar), all with K=1
  stars per row/col/region. Every puzzle is procedurally generated and
  **verified** to have exactly one solution by a row-by-row backtracking
  solver (per-row star quota + per-col / per-region cap + no-diagonal
  rule + a `c+2` next-column lower bound that prunes adjacent placements
  inside the same row).
- Random region partitioning by BFS-grown round-robin expansion from N
  seed cells — regions touch every edge and have varied shapes, but the
  generator retries any partition with regions smaller than 2 cells.
- Live red conflict highlight: any row, column, or region that has more
  than K stars flashes red; two stars that touch (orthogonal **or**
  diagonal) both flash red.
- Tap any cell to cycle **blank → ★ (star) → ✕ (eliminated) → blank**.
  The ✕ mark is for human note-taking — the rules don't care about it.
- Score = `999 - seconds` on a clean solve.
- English / 中文, `localStorage` save with cleared puzzles and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 45 data checks (every level builds, every solution wins via
  `isSolved`, every clue set has exactly one solution, every generator
  output is bit-for-bit deterministic), plus row-of-stars, adjacent-stars,
  and full-solution sanity checks for `findViolations` / `isSolved`.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4300
```

Then visit `http://127.0.0.1:4300/index.html`.

## Play

- Tap any cell — first tap places a star, second swaps it to an ✕
  (your "no star here" note), third clears the cell.
- The rules are short:
  * exactly K stars per row.
  * exactly K stars per column.
  * exactly K stars per region (the thick-outlined shapes).
  * no two stars can be 8-neighbours of each other.
- Win when all four rules are satisfied at once.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — partition generator, row-by-row solver with strict
  per-row / per-col / per-region / no-diagonal pruning, uniqueness check.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette + per-region tint, grid renderer with thick
  region borders, pixel star + ✕ sprites, HUD.
- `js/game.js` — screen flow, tap input, win check, timer, save.
