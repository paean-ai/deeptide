# Pixel Shikaku

A pixel-art **Shikaku** puzzle. Carve the grid into rectangles so every
rectangle contains exactly one number and its area equals that number. A fresh
divide-the-grid logic genre alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign on growing plots (5×5 up to 7×8).
- Each level seeds a random rectangle tiling, places one clue per rectangle,
  then a backtracking solver **verifies the puzzle has exactly one solution**
  before it ships.
- Drag from any corner to the opposite corner to outline a rectangle. Drop on
  a valid rectangle (one clue inside, area matches) to place it; drop on an
  existing rectangle to remove it.
- Live preview of the in-progress rectangle, tinted green if it would be
  accepted and red if it would not.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4259
```

Then visit `http://127.0.0.1:4259/index.html`.

## Play

- Drag a finger from one cell to the opposite corner cell to outline a
  rectangle.
- Each rectangle must contain exactly one number, and its area must match
  that number.
- Tap an already-placed rectangle to remove it.
- Cover the whole grid to win.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - random tiling, the uniqueness solver, validation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cells, clue circles, placed rectangles, preview.
- `js/game.js` - drag-to-draw input, win detection, save.
