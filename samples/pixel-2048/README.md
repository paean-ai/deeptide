# Pixel 2048

A pixel-art take on **2048** — slide every tile in a direction, two tiles
of the same number merge into one of double the value, every move spawns
a fresh 2 or 4. Reach the 2048 tile to win. A fresh slide-and-merge logic
puzzle alongside the other `samples/` pixel games.

## Features

- Classic 4×4 board with the canonical slide-then-spawn flow and the
  one-merge-per-slide rule that the original is famous for.
- Score = total of all merge values; per-run **best score** persisted to
  `localStorage`, plus mid-run save (your in-progress board survives a
  page reload).
- Single-step **Undo** (one move back) and **New Game**.
- 2048 tile triggers a **You reached 2048** modal — dismiss with *Keep
  going* to chase 4096 and beyond. The deadlock modal appears when no
  slide changes the board.
- Swipe at a 28 px threshold OR keyboard arrows / WASD / `Z` for undo.
- Pixel-art tile chips with bevel + the original 2048 colour ramp
  (warm low values → cool deep purples past 4096), bitmap-style monospace
  numerals that auto-shrink to fit longer values.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 35 mechanics checks — every slide / merge rule (1 / 2 / 3 / 4
  same in a row, gaps, distinct pairs), all four directions, score
  totalling, undo round-trip, no-op moves do not spawn, win triggers at
  2048, deadlock detection.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4320
```

Then visit `http://127.0.0.1:4320/index.html`.

## Play

- Swipe (or press an arrow / WASD key) in the direction you want every
  tile to slide.
- Adjacent tiles of the same number merge into double — only one merge
  per tile per slide.
- Reach **2048** to win the run. Keep going to chase higher tiles.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — board state, `slideRowLeft` core + the four-direction
  wrapper, spawn, deadlock check, undo snapshot.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, bevelled tile chips with auto-sizing numerals,
  HUD score / best chips.
- `js/game.js` — screen flow, swipe + keyboard input, mid-run save.
