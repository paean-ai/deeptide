# Pixel Mosaic

A pixel-art **Fill-a-Pix** / Mosaic puzzle. Every cell shows how many of the
nine cells in its 3×3 neighbourhood (itself included) should be filled. Shade
the right cells and a picture appears. A fresh count-and-fill genre alongside
the other `samples/` pixel games — mechanically distinct from the run-length
clues of `pixel-nonogram`.

## Features

- 6 hand-drawn 10×10 pictures.
- Tap any cell to cycle **filled → marked-empty → blank**; the marked-empty
  state is your own scratchpad for cells you have ruled out.
- Cells whose clue is already impossible (too many filled, or not enough room
  left to reach it) flash red so contradictions surface immediately.
- Win on satisfying every clue — the picture lights up in its own colour.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4257
```

Then visit `http://127.0.0.1:4257/index.html`.

## Play

- Each cell's number is how many cells in its 3×3 neighbourhood (including
  itself) should be filled.
- Tap a cell to cycle its state. Cells you are sure are empty can be marked
  with an ✕ so you don't accidentally shade them.
- Satisfy every clue to complete the picture.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - the 6 puzzle bitmaps and clue derivation / evaluation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, cells, clue numbers, fill / empty marks.
- `js/game.js` - tap-to-cycle play, win detection, save.
