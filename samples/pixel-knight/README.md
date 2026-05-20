# Pixel Knight

A pixel-art take on the classic **Knight's Tour** — from a fixed
starting square, hop the chess knight in L-shapes and visit every
square on the board **exactly once**. A pink Warnsdorff hint lights
up the move with the fewest onward exits, the textbook heuristic
that lets you find a tour even on the legendary 8×8 board. A fresh
single-piece chess puzzle alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Squire → Throne** scaling board size (5×5 →
  8×8) and starting position. Every level was verified at design
  time by an in-process Warnsdorff search to admit at least one
  open tour, so each tour is genuinely findable — never a dead end
  by construction.
- All eight knight-move target cells light up as blue hops; the
  **Warnsdorff hint** overlay (toggle with the **Hint** button or
  `H`) marks the candidate move with the fewest onward exits in
  pink — pick it consistently and the heuristic finds a complete
  tour.
- Tap any blue / pink target to hop there. Each landing carries
  its move number 1..N², the live HUD shows visited / total.
- Unlimited **Undo** and **Restart** at any time (`Z` / `R`).
- Star rating per level: 3 = complete tour, 2 = ≥ 80 % visited,
  1 = otherwise. Per-level highest-cells-covered record persisted
  to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 34 mechanics checks — every level admits a tour via
  Warnsdorff; `inBounds` and `legalMoves` enforce the eight knight
  offsets; corner of a 5×5 has 2 moves, centre of 8×8 has 8;
  `tryMove` rejects non-knight / already-visited / OOB attempts;
  undo restores cells and counters; restart wipes state; following
  the Warnsdorff hint on L1 completes the 25-cell tour; star tiers
  correct; the win triggers on the last cell.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4332
```

Then visit `http://127.0.0.1:4332/index.html`.

## Play

- Tap any blue ring (a knight target) to hop there.
- The **pink** ring is the Warnsdorff pick — the target with the
  fewest exits after you land. Following it consistently finishes
  the tour.
- Hit **Undo** to back out, **Restart** to reset, or toggle the
  **Hint** if you want a pure puzzle.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — knight offsets, legal-move / onward-degree /
  Warnsdorff-hint helpers, move + undo + restart, `findTour`
  verifier (used by tests).
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, chessboard with move-number labels,
  knight glyph, target + Warnsdorff hint rings, HUD, stars.
- `js/game.js` — screen flow, tap input, hint toggle, save.
