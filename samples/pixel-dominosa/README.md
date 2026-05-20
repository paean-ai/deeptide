# Pixel Dominosa

A pixel-art **Dominosa** puzzle. A grid is filled with pip numbers; draw the
domino boundaries so the whole set — every domino from 0-0 upward — appears
exactly once. A fresh tiling-deduction genre alongside the other `samples/`
pixel games.

## Features

- 12-puzzle campaign on growing fields (a double-2 set up to a double-7 set,
  the largest being an 8×9 board with all 36 dominoes from 0-0 through 7-7).
- Each level lays a random complete domino set, writes the pips, then a
  backtracking solver **verifies the pip grid has exactly one partition** — so
  every level is a genuine, uniquely-solvable Dominosa.
- Dominoes are drawn as capsules over the pips; any domino whose pip pair is
  already used elsewhere flashes red.
- A live count of uncovered squares sits in the HUD.
- Level select with progressive unlocks and per-puzzle completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4247
```

Then visit `http://127.0.0.1:4247/index.html`.

## Play

- Tap between two neighbouring squares to lay a domino across them; tap it
  again to remove it.
- Laying a domino over a square that already belongs to one replaces it.
- Every domino in the set must appear exactly once — no pip pair repeated, no
  square left uncovered.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - puzzle generation and the uniqueness-verifying solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, pip tiles, domino capsules.
- `js/game.js` - domino placement, win detection, save.
