# Pixel Thermometers

A pixel-art take on the **Thermometers** logic puzzle. The grid is
tiled by snake-shaped thermometers, each with a round bulb at one end.
Mercury always rises **from the bulb** with no gaps — deduce how far
each thermometer is filled so every row and column count matches. A
fresh deduction-puzzle genre alongside the other `samples/` pixel
games.

## Features

- 9-level campaign **Chill → Inferno** on growing grids (5×5 → 8×8).
- Every puzzle is procedurally generated and **verified unique**:
  1. A random-walk path-cover tiles the whole grid with thermometers
     (each a branch-free run of ≥ 2 cells, bulb at one end).
  2. A random fill amount per thermometer sets the hidden solution;
     row + column counts are derived from it.
  3. A backtracking solver — pruned by a bidirectional bound, so each
     line must stay reachable yet never overshoot — confirms exactly
     one fill assignment reproduces the clues.
- Tap a cell to fill its thermometer up to that point; tap the cell at
  the top of the mercury to retract it — the fill is always a single
  contiguous run from the bulb, so no illegal state is reachable.
- Live feedback: a row / column clue turns **green** when its count is
  exact and **red** when overshot, and every cell in an overfilled
  line gets a red tint.
- Score = `999 - seconds` on a clean solve; level select with
  progressive unlocks and per-level best, saved to `localStorage`.
- Chunky pixel thermometers — outlined glass tubes, round bulbs, a
  rising mercury column with a bright sheen.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first
  tap controls.
- Verified: 194 checks — every level builds with a fully-tiled,
  non-overlapping, orthogonally-contiguous thermometer layout; the
  owner / position maps are consistent; each clue set has exactly one
  solution; the authored fill solves and an empty board does not; the
  generator is deterministic; clues match the authored fill; conflict
  detection flags an overfilled board and clears on the solution; the
  tap-to-fill / tap-to-retract mechanic behaves; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4361
```

Then visit `http://127.0.0.1:4361/index.html`.

## Play

- Each thermometer fills with mercury from its round **bulb** end.
- Tap a cell to raise the mercury to that cell; tap the topmost
  mercury cell to lower it.
- Mercury is always one unbroken run — you can never leave a gap.
- The numbers around the board count the mercury cells in each row
  and column.
- Win when every row and column count is matched exactly.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — path-cover thermometer layout, the hidden fill, clue
  math, uniqueness solver, live conflict + win helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, board, outlined glass thermometers with
  bulbs and a rising mercury column, tinted clues, HUD.
- `js/game.js` — screen flow, tap input, timer, save.
