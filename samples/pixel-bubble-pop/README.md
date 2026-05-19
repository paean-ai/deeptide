# Pixel Bubble Pop

A dependency-free bubble shooter — aim the launcher, fire coloured bubbles up
the offset grid, and burst clusters of three or more. A fresh aim-puzzle genre
alongside the other `samples/` pixel games.

## Features

- Classic offset-grid bubble shooter: shots bounce off the side walls and snap
  to the grid.
- Match 3+ same-colour bubbles to pop them; bubbles cut off from the ceiling
  drop for bonus points.
- The loaded colour is always drawn from colours still on the board.
- Clear the whole board for a 500-point bonus and a fresh wave.
- Lose if the bubbles pile past the danger line.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive canvas: aim with the mouse, drag-aim on touch, or arrow keys;
  release / `Space` to shoot.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4203
```

Then visit `http://127.0.0.1:4203/index.html`.

## Play

- Point the launcher and release (or press `Space`) to fire a bubble.
- Bank shots off the walls to reach tricky spots.
- Land 3+ of a colour together to pop the cluster; isolating bubbles drops
  them too.
- Keep the grid above the danger line.

## Structure

- `index.html` - title / game screens and overlays.
- `css/style.css` - responsive UI.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - grid layout, colours, offset-grid neighbour math.
- `js/art.js` - bubble, launcher and aim-line rendering.
- `js/game.js` - shooting, snapping, cluster pops, drops, screens, save.
