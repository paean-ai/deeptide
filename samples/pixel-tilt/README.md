# Pixel Tilt

A pixel-art **slide-the-crystals puzzle**. Tilt the whole cavern and every
crystal slides that way at once — until a wall, the edge, or another crystal
stops it. Land each crystal on its matching-colour goal pad in as few tilts as
you can. A fresh tilt-and-slide puzzle genre alongside the other `samples/`
pixel games.

## Features

- 6 caverns on a rising curve — Quartz, Garnet, Amethyst, Topaz, Sapphire,
  Diamond — from a 5×5 with two crystals up to a 7×7, with par climbing from
  6 tilts to 24.
- All crystals move together: a tilt is one decision that shifts the whole
  board, and crystals stack against each other, so threading four of them to
  four goals is a real spatial puzzle.
- Every cavern's par is the genuine shortest solution — found by an offline
  breadth-first search and baked in; the test re-derives it to confirm each
  level is solvable in exactly par.
- Smooth slide animation, three-star scoring (par / a small margin / cleared),
  undo and restart, per-cavern stars and progressive unlocks.
- Swipe the board, tap the on-screen d-pad, or use the arrow keys / WASD;
  works cleanly on phone and desktop. English/中文 toggle, saved to
  `localStorage`.
- Verified: 83 checks — all 6 levels are structurally sound, a breadth-first
  search re-derives every baked par and its solution replays through real
  tilts to a win, slide / wall-stop / crystal-stacking / null-tilt / undo /
  restart behave; plus a 4-script load-and-render smoke test that drives a
  full solution to a three-star win.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4433
```

Then visit `http://127.0.0.1:4433/index.html`.

## Play

- Swipe the cavern, tap a d-pad arrow, or press an arrow key to tilt.
- Every crystal slides in that direction until a wall, the board edge, or
  another crystal stops it — they all move on the same tilt.
- Land each crystal on the goal pad of its own colour to clear the cavern.
- Undo a tilt or restart any time. Match par for three stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the baked levels, the slide rule and play state.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cavern grid, crystals, d-pad, title art.
- `js/game.js` - screen flow, swipe / d-pad input, slide animation, save.
