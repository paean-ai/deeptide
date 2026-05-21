# Pixel Numpath

A pixel-art **consecutive-number path** puzzle. Fill the grid with
1 .. N so each number sits orthogonally next to the next — one snaking
path that covers every cell. A few numbers are given; the rest you
trace yourself. A fresh path-logic genre alongside the other
`samples/` pixel games.

Every step is **orthogonal**, so the answer is a true Hamiltonian path
of the grid — distinct from a king-move number-snake.

## Features

- 6-level campaign **Spark → Odyssey** on growing grids (5×5 → 7×7).
- Every puzzle is procedurally generated and **verified unique**:
  1. A random Hamiltonian path is built by **backbite** shuffling of
     the boustrophedon snake — re-rooting an endpoint onto a random
     neighbour, a move that always keeps the path valid, so
     generation can never fail.
  2. The path numbers the cells 1 .. N; the givens are then thinned
     away at random, each removal kept only while a backtracking
     solver confirms exactly one solution remains.
  3. The numbers 1 and N stay as anchors; the rest thins to roughly
     40 % of the cells.
- Build the path by tapping: tap a cell next to the path's end to
  extend it; a given number can only be reached at exactly its step,
  so the givens steer the route. Tap any cell already on the path to
  roll the path back to there.
- The path is drawn as a glowing snake with every step numbered.
- Score = `999 - seconds` on a clean solve; level select with
  progressive unlocks and a per-level best, saved to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 66 checks — the backbite generator yields a real
  Hamiltonian path at every size; every level builds with exactly one
  solution, both anchors given, a sensible thinned clue set of
  distinct in-range numbers, and is deterministic; the start cell is
  the value-1 cell; the unique solution replays cell by cell through
  `canExtend` to a win; `canExtend` rejects a non-adjacent cell and a
  given reached at the wrong step while accepting the correct next
  cell; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4393
```

Then visit `http://127.0.0.1:4393/index.html`.

## Play

- The path starts on the **1**. Tap a cell orthogonally next to the
  path's end to extend the path by one step.
- A **given number** can only be stepped onto when the path reaches
  exactly that step — use the givens to plan the route.
- Tap a cell already on the path to **roll the path back** to it.
- Cover every cell — finishing on **N** — to clear the puzzle.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — backbite Hamiltonian-path generation, the
  consecutive-path uniqueness solver, clue thinning, play helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, board, the glowing numbered path snake, HUD.
- `js/game.js` — screen flow, tap-to-extend input, timer, save.
