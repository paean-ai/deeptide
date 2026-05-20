# Pixel Light Up

A pixel-art **Light Up** (Akari) puzzle. Place bulbs so every cell is lit, no
bulb shines on another, and every numbered wall has exactly that many bulbs
beside it. A fresh illumination logic genre alongside the other `samples/`
pixel games.

## Features

- 8-puzzle campaign on hand-built rooms, 7×7 and 8×8.
- Each puzzle takes a candidate light arrangement, numbers every wall from it,
  and a constrained backtracking solver **confirms the numbered puzzle has
  exactly one solution** — so every level is a genuine, fair Akari.
- Bulbs cast a glow down their row and column until a wall; cells lit warm,
  dark cells stay cold.
- Bulbs that shine on each other flash red; walls turn green when their number
  is met, red when overshot. A live "dark cells" counter sits in the HUD.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4246
```

Then visit `http://127.0.0.1:4246/index.html`.

## Play

- Tap a white cell to place a bulb; tap it again to remove it.
- A bulb lights its own cell and every cell along its row and column until a
  wall blocks the beam.
- No bulb may stand in another bulb's beam.
- A numbered wall must have exactly that many bulbs in the cells beside it.
- Light every cell with the numbers satisfied to win.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - layouts, puzzle generation, the uniqueness solver, evaluation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, walls, lit cells, glowing bulbs.
- `js/game.js` - bulb placement, win detection, save.
