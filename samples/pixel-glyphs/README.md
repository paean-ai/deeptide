# Pixel Glyphs

A pixel-art **Lights Out** puzzle. Press a glyph and it flips — along with its
four neighbours. Light every glyph at once to break the lock. A fresh
toggle-logic genre alongside the other `samples/` pixel games.

## Features

- 8-level campaign on growing grids (3×3 up to 7×7) — Nexus (6×6) and
  Labyrinth (7×7) cap the run.
- Each lock is scrambled by pressing a random set of glyphs from the solved
  board, so it is **always solvable by construction**.
- A GF(2) linear-algebra solver computes the true minimum-press solution for
  every level — that count is the par you race against.
- 3-star rating by how few presses you use; level select with progressive
  unlocks and per-level star records, all saved to `localStorage`.
- Glowing glyphs with a soft pulse; a tidy pixel rune stamped on each.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4249
```

Then visit `http://127.0.0.1:4249/index.html`.

## Play

- Tap a glyph to flip it and the glyphs directly above, below, left and right.
- A flip turns a dark glyph bright and a bright one dark.
- Light every glyph on the board at the same time to break the lock.
- Solve it in par presses or fewer for the full three stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - level scrambling and the GF(2) minimum-press solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, glowing glyphs, rune stamps.
- `js/game.js` - press handling, move counting, win detection, scoring, save.
