# Pixel Crate Cosmos

A dependency-free space-station box-pushing puzzle — push every glowing
power-core onto its socket to bring each station back online. A fresh
level-based puzzle genre alongside the other `samples/` pixel games.

## Features

- 14 hand-built levels of escalating difficulty, played on a tidy pixel-art
  space station.
- Slide mechanic: ice panels send the robot gliding until it hits a wall, a
  core, or normal floor — cores themselves never slide.
- Unlimited **undo** and instant **restart** — experiment freely.
- Move counter with a per-level best, plus a level-select grid.
- `localStorage` progress: unlocked levels and best move counts persist.
- English / 中文 toggle.
- Responsive square-ish canvas: keyboard (arrows / WASD), on-screen d-pad, and
  swipe-to-move on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4194
```

Then visit `http://127.0.0.1:4194/index.html`.

## Play

- **Desktop:** arrow keys / WASD to move, `Z` / `U` to undo, `R` to restart.
- **Mobile:** swipe on the station or use the d-pad; Undo / Restart buttons.
- Push power-cores (you cannot pull them) onto every socket to clear a level.
- A core shoved into a corner is stuck for good — undo and rethink.
- On ice the robot keeps sliding; use it to reach far corners.

## Structure

- `index.html` - title / level-select / game screens and overlays.
- `css/style.css` - responsive UI, level grid, d-pad.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - level layouts and the level parser.
- `js/art.js` - pixel tiles, sockets, cores, and the robot.
- `js/game.js` - move simulation, undo, save, level flow, rendering.
