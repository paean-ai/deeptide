# Pixel Tents

A pixel-art **Tents and Trees** logic puzzle. Pitch one tent next to every
tree, never let two tents touch (even diagonally), and match the row /
column counts. A fresh place-and-pair logic genre alongside the other
`samples/` pixel games.

## Features

- 6-puzzle campaign on growing forests (6×6 up to 8×8).
- Each level seeds a random pairing of trees and tents, then a most-
  constrained-first backtracking solver **verifies the row / column count
  clues have exactly one solution**.
- Tap a cell to cycle blank → tent → grass; tapping a tree does nothing.
- Cells that already break a rule (two tents touching, or a tent with no
  adjacent tree) flash red live.
- Side numbers turn green the moment that row or column has the right
  number of tents.
- Level select with progressive unlocks and per-puzzle completion marks,
  saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4265
```

Then visit `http://127.0.0.1:4265/index.html`.

## Play

- Tap a cell to cycle through blank → tent → grass.
- Every tree must have exactly one orthogonally-adjacent tent, and every
  tent must have an orthogonally-adjacent tree.
- No two tents may share even a diagonal corner.
- The number at the side of each row and column is how many tents go in it.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - tree / tent pairing generation and the uniqueness solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cells, trees, tents, grass marks, side clues.
- `js/game.js` - tap-to-cycle play, win detection, save.
