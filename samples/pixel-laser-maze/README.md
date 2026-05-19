# Pixel Laser Maze

A pixel-art laser puzzle. Drop and rotate mirrors to bend the emitter's beam
through every crystal. A fresh light-routing puzzle genre alongside the other
`samples/` pixel games.

## Features

- 8-level campaign of hand-built grids — every level is brute-force verified
  solvable within its mirror budget.
- A deterministic beam: the laser leaves the emitter, walls stop it, mirrors
  deflect it 90°, and crystals light up when the beam passes through.
- Tap a tile to cycle a mirror through `/`, `\` and removed — within a limited
  mirror budget per level.
- A glowing beam that re-routes live as you place each mirror.
- 3-star rating by how cleanly you solve it, level select with progressive
  unlocks, per-level star records — all saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4236
```

Then visit `http://127.0.0.1:4236/index.html`.

## Play

- Tap an empty tile to place a mirror; tap again to flip it, once more to
  remove it.
- The beam reflects 90° off each mirror — `/` and `\` bend it different ways.
- Route the beam so it passes through every crystal at once.
- Solve with few taps for the 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - level grids, beam tracing, the solvability checker.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, walls, emitter, crystals, mirrors, the beam.
- `js/game.js` - mirror placement, beam rendering, win detection, save.
