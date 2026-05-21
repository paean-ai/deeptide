# Pixel Slant

A pixel-art take on the classic **Slant** (Gokigen Naname) puzzle.
Draw a diagonal in every cell — either `\` or `/`. A numbered lattice
point must be touched by exactly that many diagonals, and the slashes
must never close a loop. A fresh deduction-puzzle genre alongside the
other `samples/` pixel games.

## Features

- 6-level campaign **Sketch → Labyrinth** on growing grids (5×5 →
  7×7 cells).
- Every puzzle is procedurally generated and **verified unique**:
  1. A diagonal is laid in each cell in random order — a cell takes
     whichever slash does not close a loop, so the slashes always
     form a loop-free forest and a valid solution exists.
  2. Each lattice point's clue is the count of diagonals meeting it.
  3. Clues are thinned at random — every removal kept only while a
     backtracking solver (pruned by point counts and a union-find
     loop check) confirms the puzzle still has exactly one solution.
- Tap a cell to cycle it **blank → \ → / → blank**.
- Live feedback: a clue point turns **green** when its count is exact
  and **red** when overshot; any slash that lies on a **loop** is
  drawn red — peeled out by a 2-core of the drawn graph.
- Score = `999 - seconds` on a clean solve; level select with
  progressive unlocks and a per-level best, saved to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 63 checks — every level builds with exactly one solution,
  a sane half-filled clue set of 0–4 values, and is deterministic;
  the unique solution wins `isSolved` and is loop-free while an empty
  board does not; the generator always produces a loop-free forest;
  the loop detector flags a deliberate 4-cell diamond loop and the
  point-counter tallies a diagonal's endpoints; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4401
```

Then visit `http://127.0.0.1:4401/index.html`.

## Play

- Tap a cell to cycle its diagonal: **blank → \ → / → blank**.
- A number on a lattice point is exactly how many diagonals must
  touch that point.
- Fill every cell — and make sure the slashes never enclose a loop.
- Match every number with no loops to clear the puzzle.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — loop-free solution generation, clue derivation, the
  union-find uniqueness solver, clue thinning, loop + win helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, cell grid, `\` / `/` slashes (red on a loop),
  lattice points with clue circles, HUD.
- `js/game.js` — screen flow, tap input, timer, save.
