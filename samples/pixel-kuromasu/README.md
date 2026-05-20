# Pixel Kuromasu

A pixel-art **Kuromasu** logic puzzle (also known as "Where is Black
Cells?"). Shade some cells black so each numbered cell sees exactly its
count of cells in the four directions before hitting a black cell or the
grid edge. Black cells can never touch, and the white cells must all
connect. A fresh visible-count genre alongside the other `samples/` pixel
games.

## Features

- 6-puzzle campaign on 5×5 and 6×6 grids.
- Each level seeds a random non-adjacent black placement and **derives a
  minimal hint set** — a backtracking solver verifies the puzzle is
  uniquely solvable while greedily dropping hints.
- Tap a blank cell to cycle blank → black → marked-white; tapping a
  hinted cell does nothing (hints are forced white).
- Cells that already break a rule (two black cells touching, a hint
  marked black) flash red live.
- Level select with progressive unlocks and per-puzzle completion marks,
  saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4267
```

Then visit `http://127.0.0.1:4267/index.html`.

## Play

- A number is how many cells (itself included) it can see in the four
  directions before hitting a black cell or the edge.
- Tap a blank cell to cycle blank → black → marked-white.
- Black cells must not touch orthogonally, and every white cell must
  connect to every other.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - black-cell placement, hint derivation, uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cells, hints, black squares, white marks.
- `js/game.js` - tap-to-cycle play, win detection, save.
