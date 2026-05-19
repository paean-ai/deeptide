# Pixel Word Hunt

A pixel-art word-search puzzle. Drag across the letter grid to spot every
hidden word. A fresh word-puzzle genre alongside the other `samples/` pixel
games.

## Features

- 8 themed puzzles (animals, fruit, space, ocean, colours, weather, music,
  castle) on growing grids (9×9 → 11×11).
- Words hide in all eight directions — across, down and diagonal, forwards or
  backwards — and may overlap.
- Every grid is generated deterministically from a seed: words are placed by
  an exhaustive search and the gaps filled with random letters.
- Drag from a word's first letter to its last; each found word locks in with
  its own colour and crosses off the list.
- A **hint** flashes an unfound word's location; clear a puzzle with no hints
  for the 3-star rating.
- Level select with progressive unlocks, per-puzzle star records and a timer,
  all saved to `localStorage`.
- English / 中文 interface (the word lists are English).
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4238
```

Then visit `http://127.0.0.1:4238/index.html`.

## Play

- Read the word list under the grid.
- Drag from the first letter of a word straight to its last letter — any of
  the eight directions counts.
- Found words light up and get struck off the list.
- Find them all; do it without hints for 3 stars.

## Structure

- `index.html` - shell, title / puzzle-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, puzzle grid.
- `js/data.js` - themed word lists and seeded grid generation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop and letter-tile rendering.
- `js/game.js` - drag selection, word matching, hints, win, save.
