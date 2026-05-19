# Pixel Block Drop

A pixel-art falling-block puzzle. Slot the seven tetrominoes into clean rows,
clear lines for points, and keep the stack from reaching the top.

## Features

- All seven tetrominoes with four rotation states each and basic wall kicks.
- A 7-bag randomizer (every piece appears once per bag — no droughts).
- Hold slot, a 3-piece next preview, and a ghost piece showing the landing
  spot.
- Soft drop, hard drop, and a lock delay that lets you slide a resting piece.
- Line-clear scoring (single/double/triple/tetris), a level curve that speeds
  the fall every 10 lines, and a `localStorage` best-score.
- On-screen control pad plus full keyboard support.
- English / 中文 toggle.
- Responsive 300:440 canvas — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4209
```

Then visit `http://127.0.0.1:4209/index.html`.

## Play

- Move the falling piece left / right, rotate it, and drop it to fill rows.
- A full row clears and scores; clearing four at once (a "tetris") scores most.
- Hold a piece to save it for later (once per piece).
- Touch: use the on-screen pad. Keyboard: `←` `→` move, `↑` rotate, `↓` soft
  drop, `Space` hard drop, `C` hold.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 300:440 shell, screens.
- `js/data.js` - board dimensions, tetromino shapes, scoring, gravity curve.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - background, board, pieces, panel and control-pad rendering.
- `js/game.js` - piece logic, gravity, line clears, scoring, input, save.
