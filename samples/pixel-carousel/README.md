# Pixel Carousel

A pixel-art **wrap-shift picture puzzle** (a Loopover-family game). Every row
and every column is a carousel — swipe one and its tiles cycle round, wrapping
edge to edge. The board starts scrambled; spin it back into the target
picture. A fresh wrap-shift puzzle genre alongside the other `samples/` pixel
games.

## Features

- 6 pictures on a rising curve — Spark, Drift, Quarter, Strata, Halo,
  Crossfire — from a 3×3 up to a 6×5, each a clean procedural pattern
  (rings, stripes, quadrants, bands, a cross).
- One mechanic, deep play: with no blank tile, every row and column shifts
  freely and wraps — restoring the picture is all about order of operations.
- Par is the scramble length: every level can be solved by undoing the
  scramble, and a sharp eye routes a shorter path for a better score.
- A live target preview, a pip on every tile already in place, three-star
  scoring, undo and restart, per-level stars and progressive unlocks.
- Swipe a row or column on phone; drag with the mouse on desktop. English/中文
  toggle, saved to `localStorage`.
- Verified: 58 checks — row and column shifts cycle and invert correctly, a
  full lap is a no-op, every level scrambles deterministically and is solvable
  (inverting the scramble restores the picture), win / undo / restart behave;
  plus a 4-script load-and-render smoke test that swipes a level to a
  three-star win and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4441
```

Then visit `http://127.0.0.1:4441/index.html`.

## Play

- Swipe across a row to cycle it left or right; swipe down a column to cycle
  it up or down. Tiles wrap around the edges.
- Use the target preview in the corner as your goal — a pip marks each tile
  already sitting in the right place.
- Rebuild the whole picture. Undo or restart any time.
- Match par — the scramble length — for three stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the patterns, the carousel shifts, scramble and play state.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, the board, the target preview, title art.
- `js/game.js` - screen flow, swipe input, save.
