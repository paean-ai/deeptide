# Pixel Plumber

A pixel-art pipe-laying race in the Pipe Mania tradition. Drop pipe pieces to
build a route from the water source to the drain — and finish before the water
floods the line and leaks out a dead end. A fresh real-time puzzle genre
alongside the other `samples/` pixel games.

## Features

- 8-level campaign on growing grids (7×6 → 10×9) with faster water each level.
- A live water-flow simulation: water leaves the source, follows whatever
  connected pipes it finds, and a mismatch or a missing pipe ends the run.
- You don't pick your pieces — place whatever the upcoming-piece queue hands
  you (six pieces previewed); the queue is seeded so each level is repeatable.
- Re-lay any pipe the water hasn't reached yet — the cross piece lets two
  flows pass through.
- A countdown before the water starts, then a steady tile-by-tile flow.
- 3-star rating by how few pieces you re-lay, level select with progressive
  unlocks, per-level star records — all saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4232
```

Then visit `http://127.0.0.1:4232/index.html`.

## Play

- Tap any empty (or not-yet-flooded) tile to drop the next pipe from the queue.
- Build a connected route from the green source to the gold drain.
- When the countdown ends, water flows one tile at a time — it must always
  have a matching pipe ahead of it.
- Reach the drain to win. Re-lay as few pieces as possible for 3 stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win & lose overlays.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - pipe pieces, the draw bag, and the 8-level campaign.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, pipe tiles, source and drain markers.
- `js/game.js` - placement, water-flow simulation, win / lose, save.
