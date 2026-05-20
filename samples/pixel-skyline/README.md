# Pixel Skyline

A pixel-art **Skyscrapers** puzzle. Fill the grid so every row and column
holds each number once; the clue at each side tells you how many towers you
can see looking in, with taller towers hiding shorter ones behind them. A
fresh visibility-Latin-square genre alongside the other `samples/` pixel
games.

## Features

- 6-city campaign on 4×4 and 5×5 grids.
- Each level seeds a random Latin square and derives clues for all four
  sides; a backtracking solver **verifies the puzzle has exactly one
  solution** before it ships.
- Tap a cell to cycle 0..n. Cells that already break a row/column rule flash
  red live.
- Level select with progressive unlocks and per-puzzle completion marks,
  saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4263
```

Then visit `http://127.0.0.1:4263/index.html`.

## Play

- Tap a cell to cycle through 1..n (n is the board size). Tap past n to clear.
- Every row and every column must contain each number from 1..n exactly once.
- A clue outside the grid is the number of skyscrapers visible looking inward
  from that side — a taller tower hides any shorter ones behind it.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - Latin-square generation, clue derivation, the uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, clue cells, grid cells, selection.
- `js/game.js` - tap-to-cycle play, win detection, save.
