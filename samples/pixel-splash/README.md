# Pixel Splash

A pixel-art **flood-fill paint puzzle**. Paint spreads out from the top-left
cell — each move recolours your whole splash, and it instantly swallows any
patch of that colour it now touches. Cover every paintable cell before the
move budget runs out. Stones never take paint; they just shape the route. A
fresh flood-fill puzzle genre alongside the other `samples/` pixel games.

## Features

- 6 canvases on a rising curve — Studio, Atelier, Gallery, Mural, Fresco,
  Masterwork — from a stone-free 6×6 with 4 paints up to an 11×11 of 6 paints
  threaded with 9 stones.
- Every level is generated from a seed and proven solvable: stones are placed
  only while the paintable area stays fully connected, and a greedy reference
  solver fixes the par. The move budget is that par plus three, so a clear is
  always reachable.
- Three-star scoring rewards efficient play — match the reference par for
  three stars, par + 1 for two, a clear for one.
- One-tap play: tap a paint swatch to flood, with undo and restart always to
  hand; number keys 1–6 plus U / R work on desktop.
- Per-level stars and progressive unlocks, English/中文 toggle, saved to
  `localStorage`.
- Verified: 107 checks — all 6 levels build deterministically with exactly
  their stone count and a connected canvas, the greedy solution replays
  through real moves to a win in exactly par, no-op recolours / undo /
  restart / running out of budget all behave; plus a 4-script load-and-render
  smoke test that drives a full solution to a win and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4425
```

Then visit `http://127.0.0.1:4425/index.html`.

## Play

- The splash starts as the connected patch of cells at the top-left corner.
- Tap a paint swatch: the whole splash becomes that colour and merges with
  every adjacent patch already wearing it.
- Keep flooding until the splash covers every paintable cell. Stones are
  walls — the splash flows around them.
- Finish inside the move budget; match par for three stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - level generation, the flood logic and greedy reference solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, the paint grid, swatches, HUD, title art.
- `js/game.js` - screen flow, tap / keyboard input, save.
