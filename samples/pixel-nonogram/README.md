# Pixel Nonogram

A pixel-art nonogram (also known as picross or griddler) — a logic puzzle where
the numbers along each row and column tell you the runs of filled cells. Solve
the grid and the hidden pixel picture appears. A fresh logic-puzzle genre
alongside the other `samples/` pixel games.

## Features

- 30-puzzle campaign on an escalating curve — growing grids (5×5 → 10×10), each
  hiding its own coloured pixel illustration (heart, cat, ghost, rocket…).
- Row and column clues derived automatically from each picture: every number is
  a run of consecutive filled cells in that line.
- Forgiving Nintendo-style scoring — a wrong cell is auto-crossed and counted as
  a mistake, so a puzzle can always be finished; clear it flawlessly for 3
  stars.
- Solved rows and columns dim their clues so you can see your progress at a
  glance.
- Mobile-first input — a FILL / MARK mode toggle, tap to fill, and drag to paint
  a whole run in one stroke.
- Level select with progressive unlocks, per-puzzle star records, all saved to
  `localStorage`.
- 5-cell group separators, a pixel-font clue typeface and a picture-reveal
  particle burst.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4225
```

Then visit `http://127.0.0.1:4225/index.html`.

## Play

- Each clue number is a run of filled cells, in order, with at least one gap
  between runs.
- In FILL mode, tap a cell you've deduced is filled — a wrong guess is crossed
  out and costs a mistake.
- Switch to MARK mode to cross off cells you know are empty (no penalty).
- Drag to fill or mark a whole line at once.
- Fill every cell of the picture to win. Solve with zero mistakes for 3 stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, control bar, level grid.
- `js/data.js` - the 30 puzzle bitmaps and clue derivation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - palette, pixel-font clue digits, cell rendering.
- `js/game.js` - clue logic, grid input, win detection, scoring, save.
