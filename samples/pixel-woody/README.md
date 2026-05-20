# Pixel Woody

A pixel-art take on the modern mobile classic **Wood Block Puzzle** —
the Woody / Block-Puzzle genre. Pieces appear in a tray of three; drop
them onto an 8×8 grid; any row OR column that fills entirely clears.
When none of the three pieces on offer fit anywhere on the board, the
run ends. A fresh place-and-clear puzzle alongside the other
`samples/` pixel games.

## Features

- **8 × 8 grid** and a **three-piece tray** drawn from a bank of 16
  polyomino archetypes (1×1 / 1×2 / 1×3 / four L-corners / 2×2 /
  1×4 / L4 / J4 / T4 / 3×3 square), weight-tuned so smaller pieces
  appear more often and the board stays playable.
- Place into the grid by tap-then-tap (tap a tray piece to pick up,
  tap a board cell to drop) — with a live ghost preview (green if
  the piece fits at that anchor, red otherwise).
- **Line clear** triggers on any full row OR column — both axes count.
  Scoring: cells placed + a **streak bonus** of 80 per cleared line
  plus an extra 20 per extra line in the same placement (so two
  lines on one drop = 80 + 100).
- The tray **refills to three** whenever all three pieces are placed.
  When none of the three new (or current) pieces can fit anywhere on
  the grid, **game over**.
- Per-run best score persisted to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 80 mechanics checks — every shape has 1..9 cells, empty
  grid accepts every shape and rejects every shape on a full grid,
  placement rejects out-of-bounds and overlap, single-row + single-
  column + dual clears each register the right count, scoring math
  for 1-line clear is exactly +82 (= 2-cell piece + 80 streak),
  tap-then-tap selects / cancels / drops correctly, and game-over
  triggers precisely when no remaining tray piece can fit on the
  current grid.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4330
```

Then visit `http://127.0.0.1:4330/index.html`.

## Play

- Tap a piece in the tray (or press **1 / 2 / 3** on desktop) to pick
  it up. The selected piece glows.
- Tap a board cell to drop the piece. A green ghost shows where it
  will land; red means it won't fit.
- Fill a full row or column to clear it. Chain multiple lines on the
  same drop for streak bonus points.
- When none of the three pieces on offer can fit anywhere on the
  board, the run ends.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — shape bank with weighted picks, place / clear / fit-
  anywhere rules, tray refill + game-over flow.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, wood-grain backdrop, bevelled grid cells,
  per-shape colour ramp, tray slots with selection ring, ghost
  preview, HUD.
- `js/game.js` — screen flow, tap-then-tap input + 1 / 2 / 3 select,
  RAF loop, save.
