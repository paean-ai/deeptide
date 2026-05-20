# Pixel Suguru

A pixel-art take on **Suguru** (also called *Tectonic* or *Sukibun*) — a
number-logic puzzle where the grid is partitioned into irregular regions and
you fill each region of size *n* with the digits 1..*n*, with the extra rule
that identical digits can never touch (orthogonally **or** diagonally). A
fresh logic-puzzle entry alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign (3 × 5×5 + 3 × 6×6). Every puzzle is procedurally
  generated and **verified** by a backtracking enumerator to have exactly one
  solution, so it can always be solved by pure deduction.
- Region partitioning algorithm grows random shapes of size 2..5, merges any
  leftover singletons into the smallest neighbour, and retries until every
  region is in the legal 2..5 range.
- Live conflict highlight: any digit that breaks the same-region or the
  8-adjacent rule flashes red the instant you place it.
- Pencil-mark NOTES mode — leave small candidate digits in a cell without
  committing.
- Selected-cell peer highlight — every cell in the same region and every
  cell showing the same digit dims up so you can sweep the board fast.
- Mistake counter (placing a digit that doesn't match the unique solution),
  on-board timer, score = `999 - seconds - mistakes * 30`.
- Number pad sized to the puzzle (5 digits on 5×5 boards, 6 on 6×6).
- English / 中文, `localStorage` save with cleared puzzles and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4290
```

Then visit `http://127.0.0.1:4290/index.html`.

## Play

- Tap a cell, then tap a digit (1..*n*) on the number pad.
- Toggle **NOTES** to leave pencil marks; toggling it back fills with
  full digits again.
- **ERASE** clears a cell (and its pencil marks).
- The same digit cannot be next to another copy of itself in any of the
  eight surrounding cells, even when they belong to different regions.
- Solve every region with no conflicts to win. Keyboard shortcuts on
  desktop: digits **1..9**, **Backspace** to erase, **N** to toggle notes.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — partition generator, region fill, solveCount uniqueness
  verifier, greedy clue trim, live validation helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, grid rendering with thick region borders and
  alternating-region tint, peer/same-digit highlight, number pad.
- `js/game.js` — screen flow, tap + keyboard input, timer, save.
