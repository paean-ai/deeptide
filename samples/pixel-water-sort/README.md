# Pixel Water Sort

A pixel-art water-sort puzzle. Pour coloured liquid between tubes until every
tube holds just one colour. A fresh sorting-puzzle genre alongside the other
`samples/` pixel games.

## Features

- 32-level campaign with a rising colour count (3 → 10 colours, plus two
  spare tubes).
- Every puzzle is reverse-scrambled from a solved board **and** confirmed by a
  built-in solver, so each level is guaranteed winnable; seeds make levels
  repeatable.
- True water-sort rules — a pour moves the whole top run of one colour onto an
  empty tube or onto a matching colour with room.
- Unlimited **undo** and a one-tap **restart**.
- 3-star rating by how few undos you lean on, level select with progressive
  unlocks, per-level star records — all saved to `localStorage`.
- Tap-to-select, tap-to-pour controls; the selected tube lifts and legal
  targets glow.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4235
```

Then visit `http://127.0.0.1:4235/index.html`.

## Play

- Tap a tube to pick it up, then tap another tube to pour.
- The top run of one colour pours across — onto an empty tube, or onto the
  same colour if there's room.
- Sort every tube into a single colour to win.
- Solve a level without undoing for the 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - colours, level seeds, puzzle generation and the solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop and glass-tube rendering.
- `js/game.js` - pour logic, undo, win detection, scoring, save.
