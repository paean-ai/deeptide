# Pixel Unblock

A pixel-art sliding-block puzzle in the Rush Hour tradition. Slide the jammed
blocks aside to free the red block out the right edge. A fresh logic-puzzle
genre alongside the other `samples/` pixel games.

## Features

- 8-level campaign of hand-built 6×6 jams on an escalating curve.
- Every level is solved by a built-in breadth-first solver, which both
  guarantees it's winnable and sets the **par** move count.
- Each block slides only along its own lane and stops at walls and other
  blocks — drag it as far as the path allows.
- 3-star rating against par, unlimited **undo**, one-tap **restart**.
- Level select with progressive unlocks and per-level star records, saved to
  `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4237
```

Then visit `http://127.0.0.1:4237/index.html`.

## Play

- Drag a block along its lane — horizontal blocks slide sideways, vertical
  blocks slide up and down.
- Blocks can't overlap or leave the board (except the red one).
- Clear the red block's row so it can slide out the exit on the right.
- Solve it in par moves for the 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - level grids, block parsing and the BFS solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, board frame, blocks.
- `js/game.js` - block dragging, win detection, scoring, save.
