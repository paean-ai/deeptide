# Pixel Armada

A pixel-art take on **Battleship Solitaire** (also called **Bimaru**) —
deduce where the fleet sits given the row + column ship counts and a
handful of revealed cells. A fresh deductive placement puzzle alongside
the other `samples/` pixel games.

## Features

- 6-puzzle campaign on growing grids (5×5 → 7×7) with progressively
  larger fleets — from a five-ship skiff to a ten-ship armada.
- Every puzzle is procedurally generated and **verified** to have exactly
  one solution by a ship-by-ship backtracking solver. The solver:
  * tries each ship in descending size order at every valid position;
  * prunes by the row / column ship-count caps, the no-touching rule,
    and any water hints;
  * deduplicates equal-sized ships with a lexicographic `code <= prev`
    guard so the two cruisers don't double-count their swap.
- Greedy hint reveal: the generator first checks the puzzle is unique
  with no hints, then reveals ship cells one at a time until uniqueness
  is reached. Small fleets often end up with **zero** hints.
- Live red conflict highlight: row / column over-counts and any
  diagonal-touching ship cells flash red as soon as you place them.
- Tap any non-hint cell to cycle **blank → ship → water → blank**. Hint
  cells are locked (yellow corner tag).
- Score = `999 - seconds` on a clean solve.
- Fleet card at the bottom of the board shows the full ship roster so
  you always know what's left to place.
- English / 中文, `localStorage` save with cleared puzzles and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 51 data checks — every level builds with the correct ship
  cell count, every solution wins `isSolved`, every clue set has
  exactly one solution, every generator output is bit-for-bit
  deterministic; plus diagonal-touch and row-overcount conflict cases.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4302
```

Then visit `http://127.0.0.1:4302/index.html`.

## Play

- Tap a cell to cycle through **blank → ship segment → water → blank**.
- Numbers above and beside the board count the ship cells in that line.
- Ships are straight lines (horiz or vert). No two ships may share an
  edge or even a corner.
- Hints (gold corner tag) are locked into the position you see.
- Win when every ship is correctly placed and no conflicts remain.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — fleet placement, ship-by-ship solver, uniqueness check,
  greedy hint reveal, live violation + win helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, sea / ship / water cells with end-caps and
  body shading per ship segment, row + column count labels, HUD, fleet
  card at the bottom of the board.
- `js/game.js` — screen flow, tap input, timer, save.
