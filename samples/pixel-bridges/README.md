# Pixel Bridges

A pixel-art **Bridges** puzzle (Hashiwokakero). Connect every numbered island
with bridges so each island carries exactly its number — without crossing.
A fresh connect-the-islands logic genre alongside the other `samples/` pixel
games.

## Features

- 8-puzzle campaign on growing seas (7×7 → 10×10), 8 to 17 islands.
- Each puzzle grows a connected island network from a seed, then a backtracking
  solver **verifies it has exactly one solution** — so every level is a
  genuine, uniquely-solvable bridges puzzle.
- Tap a corridor to lay a bridge, tap again for a double span, once more to
  clear; bridges that would cross an existing one are blocked.
- Islands turn green the moment their bridge count is right, red if overshot;
  a live solved-island counter sits in the HUD.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4245
```

Then visit `http://127.0.0.1:4245/index.html`.

## Play

- Tap between two islands to build a bridge; tap again for a double bridge,
  once more to remove it.
- Bridges run only straight across or down and may not cross each other.
- Each island's number is how many bridge ends it must have.
- Connect every island into one network with all numbers satisfied to win.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - puzzle generation and the uniqueness-verifying solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, islands, bridges.
- `js/game.js` - bridge cycling, crossing rules, win detection, save.
