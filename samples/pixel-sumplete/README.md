# Pixel Sumplete

A pixel-art take on **Sumplete** — the keep-or-delete sum puzzle. A grid
holds positive integers; you decide which cells to **KEEP** and which
to **DELETE** so the kept-cell sum in every row matches the row target
and the kept-cell sum in every column matches the column target. A
fresh decision-logic puzzle alongside the other `samples/` pixel games.

## Features

- 6-puzzle campaign on 4×4 → 6×6 grids (**Spark → Crucible**).
- Every puzzle is procedurally generated and **verified** to have
  exactly one solution by a row-by-row subset solver that enumerates
  every subset of each row whose sum equals the row target and prunes
  by column over-sum.
- Random grid of integers 1..n + a random keep-mask drives the
  targets; the generator retries seeds until the resulting row/col
  targets pin the keep-mask uniquely. All six levels build first try.
- Live red conflict highlight: any row whose KEEP sum exceeds the row
  target *or* whose KEEP + UNDECIDED sum is already less than the
  target flashes the relevant cells (and same for columns).
- Tap any cell to cycle **undecided → keep (green) → delete (struck-
  through) → undecided**.
- Score = `999 - seconds` on a clean solve.
- English / 中文, `localStorage` save with cleared puzzles + best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 191 data checks — every level builds, every grid digit is
  in `[1, n]`, every clue set has exactly one solution, generator
  output is bit-for-bit deterministic, the real keep-mask wins
  `isSolved`, plus a row-overcount conflict case.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4314
```

Then visit `http://127.0.0.1:4314/index.html`.

## Play

- Tap any cell to cycle through **undecided → keep → delete → undecided**.
- KEPT cells (green) sum in each row must equal the **yellow** row
  target on the right; same for the **green** column targets on top.
- Win when every cell is decided and every row + column sum matches.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — random grid + keep-mask generator, row-by-row subset
  uniqueness solver with column-cap pruning, live violation + win
  helpers.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, grid cells with keep/delete tints + struck-
  through digits, row/col target chips, HUD.
- `js/game.js` — screen flow, tap-to-cycle input, timer, save.
