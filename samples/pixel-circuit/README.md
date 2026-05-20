# Pixel Circuit

A pixel-art **rotate-the-wires** network puzzle (a Net / Pipes-rotation game).
Every tile is a wire piece; rotate them so power flows from the cell to every
node with no loose ends. A fresh routing-puzzle genre alongside the other
`samples/` pixel games.

## Features

- 8-board campaign on growing grids (4×4 up to 8×8) — Datacenter
  (7×7) and Megagrid (8×8) cap the run.
- Each board is grown as a random **spanning tree** of the grid, so a fully-
  connected, leak-free solution is always reachable — no solver needed, just
  rotate the wires back into place.
- Powered tiles glow green; loose stubs that point at a wall or a mismatched
  neighbour flash red so you can see where the leak is.
- Par equals the minimum rotations to put every piece back in its solved
  orientation; 3-star rating against that par.
- Level select with progressive unlocks and per-board star records, saved to
  `localStorage`.
- Smooth rotation animation; a glowing gold core marks the power source.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4253
```

Then visit `http://127.0.0.1:4253/index.html`.

## Play

- Tap a tile to rotate its wires 90° clockwise.
- Every wire end must meet another wire end — no stubs pointing at a wall or
  into an empty neighbour.
- Connect every tile to the gold power source to win.
- Match par rotations for the full three stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - spanning-tree generation, rotation and evaluation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, board panel, wire tiles, power source.
- `js/game.js` - tap-to-rotate play, animation, win detection, save.
