# Pixel Mate

A pixel-art **chess mate-in-one** puzzle pack — six hand-designed
positions where exactly White's next move delivers checkmate. The
data layer ships a faithful chess engine (pawn single + double push +
diagonal capture, knight L, bishop / rook / queen sliders, king
one-step) and the test harness verifies every position is genuinely
mate-in-one. A fresh chess-tactics puzzle alongside the other
`samples/` pixel games.

## Features

- 6-puzzle pack **Back Rank → Corner** sampling the classic mating
  patterns — back rank, queen + king, knight smother, ladder, eighth-
  rank battery, queen-and-king corner.
- Faithful chess move generation with proper check + checkmate
  detection. Pseudo-legal moves are filtered through "would leave
  my own king in check"; a wrong move undoes itself so the player
  can keep trying without losing track.
- Tap-then-tap input: tap a White piece to highlight it (yellow ring)
  and its legal targets (green rings); tap a green ring to drop.
- 3-star scoring: first try = 3, ≤ 3 tries = 2, otherwise 1. Per-
  puzzle fewest-tries record persisted to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 29 mechanics checks — pawn moves (forward, blocker,
  diagonal capture), knight L (centre = 8, corner = 2), bishop
  diagonals (centre = 13, blocked-by-own-piece), rook orthogonals
  (corner = 14), king centre (8), `inCheck` detects a rook attack
  but not the other side, and **every one of the six puzzles is
  end-to-end verified mate-in-one** by enumerating all legal White
  moves and counting mating replies; the tap-then-tap flow rejects
  wrong targets, undoes wrong moves, and triggers the win on a true
  mate; star tiers map correctly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4334
```

Then visit `http://127.0.0.1:4334/index.html`.

## Play

- Tap a White piece to select it; its legal squares glow green.
- Tap a green square to drop. A wrong move undoes itself so you
  can keep searching.
- Find the mate on the first try for three stars.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — minimal chess engine (pseudo-legal moves,
  `inCheck`, `legalMovesFor`, `isCheckmate`) + six hand-designed
  positions verified mate-in-one by the test harness.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, chessboard, Unicode-glyph piece sprites
  with outline for white pieces, selection + target rings, HUD,
  stars.
- `js/game.js` — screen flow, tap-then-tap input, save.
