# Pixel Trail

A pixel-art **Hidato** / number-snake puzzle. Numbers 1..N form a path through
every cell, with consecutive numbers on orthogonal neighbours. Some cells are
already revealed — fill in the rest. A fresh sequence-path genre alongside the
other `samples/` pixel games.

## Features

- 6-level campaign on growing grids (5×5 up to 6×6).
- Each level builds a random Hamiltonian path of the grid and reveals just
  enough numbers as clues for the puzzle to have **exactly one solution**,
  verified by a backtracking solver before it ships.
- Tap a cell next to the last filled one to extend the trail — it takes the
  next number automatically.
- Tap any earlier cell on the trail to backtrack to it.
- Level select with progressive unlocks and per-level completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4255
```

Then visit `http://127.0.0.1:4255/index.html`.

## Play

- The cell with `1` is your starting tip.
- Tap a cell next to the tip to extend the path; the new cell becomes the
  next number.
- A revealed clue forces you to be on the right cell at the right number —
  plan your route to hit every clue in sequence.
- Tap any earlier cell on your trail to backtrack to it.
- Fill every cell to complete the trail.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - Hamilton-path generation, clue-set search, the verifying solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, grid, trail, clue cells, tip marker.
- `js/game.js` - tap-to-extend / backtrack play, win detection, save.
