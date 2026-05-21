# Pixel Twirl

A pixel-art **rotate-the-blocks picture puzzle** (the Twiddle puzzle). Tap a
2×2 junction and that block of four tiles spins a quarter turn. The board
starts scrambled — twirl it back into the target picture. A fresh
rotation-puzzle genre alongside the other `samples/` pixel games.

## Features

- 6 pictures on a rising curve — Pinwheel, Eddy, Vortex, Cyclone, Maelstrom,
  Galaxy — from a 4×4 up to a 6×6, each a clean procedural pattern.
- One mechanic, deep play: every 2×2 block overlaps its neighbours, so a
  twirl ripples — restoring the picture is all about order and direction.
- Tap **SPIN** to flip the turn between clockwise and anti-clockwise.
- Par is the scramble length: every level can be solved by undoing the
  scramble, and a sharp eye finds a shorter route for a better score.
- A live target preview, a pip on every tile already in place, three-star
  scoring, undo and restart, per-level stars and progressive unlocks.
- Tap on phone, click on desktop; English/中文 toggle, saved to
  `localStorage`.
- Verified: 50 checks — block rotation is correct (four turns return to
  start, clockwise and anti-clockwise invert), every level scrambles
  deterministically and is solvable (inverting the scramble restores the
  picture), win / undo / restart behave; plus a 4-script load-and-render
  smoke test that twirls a level to a three-star win and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4449
```

Then visit `http://127.0.0.1:4449/index.html`.

## Play

- Tap a 2×2 junction — the dots between four tiles — to spin that block a
  quarter turn.
- Tap **SPIN** to switch the turn direction; ↻ is clockwise, ↺ anti.
- Use the target preview in the corner as your goal — a pip marks each tile
  already in the right place.
- Rebuild the whole picture. Undo or restart any time; match par for three
  stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the patterns, block rotation, scramble and play state.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, the board, the target preview, title art.
- `js/game.js` - screen flow, tap-to-rotate input, save.
