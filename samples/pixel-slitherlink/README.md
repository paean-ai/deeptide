# Pixel Slitherlink

A pixel-art take on the classic **Slitherlink** loop puzzle. Draw
segments along the lattice so they form **one single closed loop** —
no branches, no stray pieces — where every numbered cell is bordered
by exactly that many segments. A fresh loop-logic genre alongside the
other `samples/` pixel games.

## Features

- 6-level campaign **Moat → Serpentine** on growing lattices (5×5 →
  7×7 cells).
- Every puzzle is procedurally generated and **verified unique**:
  1. A random simply-connected blob of cells is grown; its boundary is
     one closed loop, so a solution exists by construction.
  2. Every cell's full clue (0–3) is derived from that loop.
  3. Clues are thinned away at random — each removal kept only if an
     edge-backtracking solver confirms the puzzle still has exactly
     one single-loop solution — down to roughly half the cells.
- The solver decides edges in **cell-major order** so clue pruning
  fires immediately — uniqueness verification stays sub-millisecond
  even at 7×7.
- Tap an edge to cycle it **blank → line → cross → blank**; the cross
  is a deduction aid marking an edge the loop can't use.
- Live feedback: a cell number turns red the instant too many lines
  touch it, and dims once it is satisfied; lattice dots brighten where
  the loop runs.
- Score = `999 - seconds` on a clean solve; level select with
  progressive unlocks and a per-level best, saved to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, edge-tap
  controls tuned for touch.
- Verified: 62 checks — every level builds with exactly one solution
  and a sane half-filled clue set of 0–3 values; the unique solution
  wins `isSolved` while an empty board does not; cross marks on the
  off-edges still count as solved; the single-loop tracer accepts a
  real loop and rejects an open path and the empty set; an overfilled
  cell is flagged; the generator is deterministic; plus a UI smoke
  pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4385
```

Then visit `http://127.0.0.1:4385/index.html`.

## Play

- Tap a lattice edge to cycle it: **blank → line → cross → blank**.
- A number tells you exactly how many of that cell's four edges are
  part of the loop; a blank cell can have any number.
- Every line you draw must join into **one** closed loop — no
  branches, no separate loops, nothing left dangling.
- Close the loop with all numbers satisfied to clear the puzzle.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — blob-boundary loop generation, clue derivation, the
  cell-major edge-backtracking uniqueness solver, clue thinning, live
  helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, lattice dots, clue numbers, loop lines and
  cross marks, HUD.
- `js/game.js` — screen flow, edge-tap input, timer, save.
