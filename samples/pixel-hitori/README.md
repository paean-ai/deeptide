# Pixel Hitori

A pixel-art **Hitori** puzzle. Shade cells so no number repeats in any row or
column, shaded cells never touch orthogonally, and every unshaded cell forms a
single connected island. A fresh shade-the-duplicates logic genre alongside
the other `samples/` pixel games.

## Features

- 9-puzzle campaign on growing grids (5×5 up to 8×8).
- Each level seeds a Latin square, sprinkles a sparse pattern of duplicates,
  then a backtracking solver **verifies the puzzle has exactly one valid
  shading**.
- Tap a cell to cycle blank → shaded → marked-open; the marked-open state is
  a scratchpad for cells you've decided are clean.
- Live red-highlight on cells that already break a rule (duplicates left in a
  row/column, or two shaded cells touching).
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4261
```

Then visit `http://127.0.0.1:4261/index.html`.

## Play

- Tap a cell to shade it. Tap again to mark it as open (a hint to yourself),
  and once more to clear.
- After shading, no number may repeat in any row or column.
- Shaded cells must not be orthogonally adjacent.
- Every unshaded cell must connect to every other unshaded cell.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - Latin-square + duplicate generation, the uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cells, numbers, shading.
- `js/game.js` - tap-to-cycle play, win detection, save.
