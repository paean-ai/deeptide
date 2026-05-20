# Pixel Sudoku

A pixel-art Sudoku. Fill the 9×9 grid so every row, column and 3×3 box holds
the digits 1–9. A fresh number-logic puzzle genre alongside the other
`samples/` pixel games.

## Features

- 24-puzzle campaign across three difficulty tiers (Easy / Medium / Hard).
- Every puzzle is generated to have **exactly one solution** — cells are dug
  out only while a solution-counting solver confirms uniqueness; seeded so each
  level is the same puzzle every time.
- Pencil-mark **notes** mode — toggle it to pepper cells with candidate digits;
  placing a real number clears its notes from the row, column and box.
- Live helpers — tap a cell to highlight its peers and every matching digit;
  the number pad shows how many of each digit remain.
- A **hint** drops the correct digit into a cell; mistakes are flagged in red
  and counted.
- 3-star rating — solve with no mistakes and no hints for the perfect score.
- Level select with progressive unlocks, per-level star records, all saved to
  `localStorage`.
- Pixel-font digits, 3×3 box separators, a clean dark board.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4231
```

Then visit `http://127.0.0.1:4231/index.html`.

## Play

- Tap a cell, then tap a digit on the number pad to fill it.
- Toggle NOTES to add or remove pencil marks instead of final digits.
- ERASE clears the selected cell; HINT solves one cell for you.
- A digit that breaks the puzzle turns red and counts as a mistake.
- Complete the grid with no mistakes or hints for 3 stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, control bar, level grid.
- `js/data.js` - level seeds, puzzle generation and the uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop and the pixel-font digits.
- `js/game.js` - board state, input, notes, hints, win detection, save.
