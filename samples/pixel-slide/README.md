# Pixel Slide

A pixel-art sliding-tile puzzle (the classic 15-puzzle). Slide the numbered
tiles back into order — and watch the colour gradient snap into place. A fresh
sliding-puzzle genre alongside the other `samples/` pixel games.

## Features

- 8-level campaign across growing grids — 3×3, 4×4 and 5×5.
- Every board is scrambled with random legal slides from the solved state, so
  it is always solvable; seeds make each level reproducible.
- Tiles are tinted by their *home* position, so the solved board forms a smooth
  colour gradient and a misplaced tile shows the wrong hue at a glance.
- Smooth slide animation, a move counter and a par target.
- 3-star rating against par, one-tap restart, level select with progressive
  unlocks, per-level star records — all saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4242
```

Then visit `http://127.0.0.1:4242/index.html`.

## Play

- Tap any tile in the same row or column as the gap, next to it, to slide it
  into the gap.
- Work the numbers back to 1, 2, 3 … with the gap ending bottom-right.
- Solve it in par moves for the 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - levels, scramble and slide rules.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop and gradient-tinted tiles.
- `js/game.js` - tile sliding, animation, win detection, scoring, save.
