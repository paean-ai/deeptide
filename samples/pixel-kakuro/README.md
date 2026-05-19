# Pixel Kakuro

A pixel-art **Kakuro** — the cross-sum number puzzle. Fill the white cells so
every across and down run adds up to its clue, with no digit repeated inside a
run. A fresh cross-sum puzzle genre alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign on growing grids.
- Every puzzle is generated from a seed and then **machine-verified to have
  exactly one solution** — a backtracking solver confirms uniqueness before the
  puzzle ships, so each level is a genuine, fair kakuro.
- Clue cells show the across and down sums split by a diagonal, just like a
  printed kakuro.
- Duplicate digits and finished-but-wrong runs are flagged in red as you play.
- A 1-9 digit pad and an erase key — tap a white cell, then tap a digit.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4244
```

Then visit `http://127.0.0.1:4244/index.html`.

## Play

- Tap a white cell to select it, then tap a digit on the pad to fill it.
- Each across / down run must total the clue in its wall cell, and no digit may
  repeat within a run.
- Red cells mean a run has a repeat or is full with the wrong total — fix them.
- Fill every cell correctly to solve the puzzle.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - puzzle shapes, seeded generation, the uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, walls, clue cells, white cells.
- `js/game.js` - puzzle play, digit pad, run validation, win detection, save.
