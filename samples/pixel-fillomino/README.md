# Pixel Fillomino

A pixel-art **Fillomino** puzzle — the region-size logic classic. Fill every
cell with a number: a cell holding **N** belongs to a block of exactly **N**
joined cells all holding N, and two blocks of the same size may never touch
edge to edge. A fresh region-logic genre alongside the other `samples/` pixel
games.

## Features

- A 6-field campaign — Cottage, Hamlet, Orchard, Township, Borough, Citadel —
  on growing grids from 5×5 up to 7×7.
- Each field is generated and **uniqueness-verified offline** by a
  backtracking region solver, then baked in; the test re-derives that every
  clued grid has exactly one solution.
- Live region borders draw themselves as you fill, and any block that breaks
  the rules — too big, or sealed in too small — flashes red at once.
- Tap a cell then a number, or use the number keys; erase and restart are
  always to hand.
- Per-field completion marks and progressive unlocks, English/中文 toggle,
  saved to `localStorage`.
- Verified: 52 checks — every baked field is a valid Fillomino whose clues
  are a true subset and yield exactly one solution, the violation detector
  flags oversized and sealed-in blocks, clued cells stay locked, filling the
  solution wins; plus a 4-script load-and-render smoke test that solves a
  field by tapping and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4457
```

Then visit `http://127.0.0.1:4457/index.html`.

## Play

- Tap a blank cell to select it, then tap a number to fill it.
- A number N must end up in a block of exactly N orthogonally-joined cells
  all showing N.
- Two separate blocks of the same size may not share an edge.
- Fill every cell with no rule broken to map the field.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the baked fields, play state and the violation detector.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, the grid with live region borders, number pad.
- `js/game.js` - screen flow, tap / keyboard input, save.
