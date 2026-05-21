# Pixel Norinori

A pixel-art take on **Norinori** — a Nikoli domino-shading logic puzzle.
The grid is partitioned into regions; shade exactly **two cells per
region** so they form a `1×2` domino, and no two dominoes from different
regions can share an edge. A fresh placement-logic puzzle alongside the
other `samples/` pixel games.

## Features

- 9-puzzle campaign on growing grids (5×5 → 8×8) with 5–10 regions each.
- Every puzzle is procedurally generated and **verified** to have exactly
  one solution by a region-by-region backtracking solver that picks one
  domino per region and prunes by:
  * the no-touching-other-domino rule (per-cell orthogonal-adjacency).
  * any hint cells (shaded or empty) that have been revealed.
- Greedy hint trim: the generator starts with **every** cell revealed,
  then removes hints one at a time while uniqueness still holds.
  Resulting puzzles ship with only 3–8 hints out of 25–49 cells.
- Yellow corner tags mark hint cells (gold = shaded hint, pale = empty
  hint); hint cells are locked and can't be changed.
- Live red conflict highlight:
  * Two shaded cells in different regions sharing an edge.
  * A region with more than two shaded cells.
  * A region with exactly two shaded cells that are *not* orthogonally
    adjacent (i.e., not a domino).
- Tap any non-hint cell to cycle **blank → shaded → empty-dot → blank**.
- Score = `999 - seconds` on a clean solve.
- English / 中文 toggle, `localStorage` save with cleared puzzles + best
  scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 55 data checks — every level builds with the right region
  count, every solution has exactly `2k` shaded cells, every clue set
  has exactly one solution, generator output is bit-for-bit deterministic;
  plus 3-shaded-in-region and 2-non-adjacent-shaded conflict cases.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4306
```

Then visit `http://127.0.0.1:4306/index.html`.

## Play

- Tap any cell to cycle through **blank → shaded → empty-dot → blank**.
- Each thick-outlined region needs to hold exactly one 1×2 shaded
  domino.
- Dominoes from different regions cannot touch along an edge (corners
  are fine).
- Win when every region has its domino and no rules are broken.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — partition generator, per-region domino enumeration,
  region-by-region uniqueness solver, greedy hint trim, live violation
  + win helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette + per-region tint, shaded / dot / hint sprites,
  thick region borders, HUD.
- `js/game.js` — screen flow, tap input, timer, save.
