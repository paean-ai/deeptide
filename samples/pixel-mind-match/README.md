# Pixel Mind Match

A dependency-free memory pairs game with a level campaign — flip tiles, memorise
the pixel creatures, and match every pair in as few moves as you can. A fresh
puzzle genre alongside the other `samples/` pixel games.

## Features

- Classic flip-and-match memory play with crisp horizontal card-flip animation.
- 24 distinct pixel creatures (6 shapes x 4 colour variants) so even the largest 6 x 8 board never repeats a sprite.
- 9 levels with growing grids (3x4 up to 6x8) and a level-select screen.
- A short **memorise** preview shows the board face-up at the start of a level.
- Move counter, consecutive-match combo, per-level best moves and a 1-3 star
  rating.
- `localStorage` progress — unlocked levels and best move counts persist.
- English / 中文 toggle.
- Responsive square canvas, fully playable with taps on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4202
```

Then visit `http://127.0.0.1:4202/index.html`.

## Play

- The board flashes face-up for a moment — memorise it.
- Tap two tiles to flip them; a matching pair stays lit, a mismatch flips back.
- Clear every pair to win the level; fewer moves earn more stars.
- Chain matches without a miss to build a combo.

## Structure

- `index.html` - title / level-select / game screens and overlays.
- `css/style.css` - responsive UI, level grid.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - creature set and level grids.
- `js/art.js` - card back and creature face rendering.
- `js/game.js` - flip / match logic, levels, save, screen flow.
