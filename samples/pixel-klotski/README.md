# Pixel Klotski

A pixel-art take on **Klotski** (Chinese: 华容道 *Huarong Trail*) —
escort the red 2×2 general past the soldiers blocking the 4×5 board to
the exit slot at the bottom-centre. Each block slides one cell at a
time into empty space and never rotates. A fresh sliding-block logic
puzzle distinct from `pixel-unblock` (Rush Hour cars).

## Features

- 6-level campaign **Foothold → Huarong** with rising piece counts
  (4 → 10) and BFS-derived pars of **10 / 13 / 16 / 20 / 54 / 116**
  single-cell slides — the listed par is the proven optimum from an
  in-process BFS over canonicalised (shape-grouped) board states,
  so a 3-star clear means you matched the perfect solution.
- Four block shapes — 1×1 soldier, 1×2 / 2×1 lieutenant, and the
  2×2 general (target). The general is distinctly red with a centre
  glyph so it always reads at a glance; other shapes use a
  per-shape colour ramp.
- **Tap-select + drag-to-slide** input — tap a block to highlight it
  (yellow ring), then drag a short stroke in the slide direction.
  Arrow keys / WASD also work on desktop; **Z / R** undo / restart.
- Unlimited **Undo** and **Restart** at any time.
- Star rating per level: 3 = matched par, 2 = within +50 %,
  1 = solved-but-loose. Per-level **lowest-move** record persisted
  to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 42 mechanics checks plus a full 6-level BFS solvability
  proof — every level builds with no piece overlaps + every piece in
  bounds + piece 0 is the 2×2 general; the BFS solver confirms each
  level's listed par is the actual minimum; tap-select / re-tap to
  cancel; slides respect the no-overlap + in-bounds rules; undo
  round-trip; restart wipe; isWin recognises the general at goal;
  pieceAt finds the block at any cell; star tiers correct.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4328
```

Then visit `http://127.0.0.1:4328/index.html`.

## Play

- Tap a block to select it (yellow ring lights up).
- Drag a short stroke (or press an arrow key / WASD) in the direction
  you want it to slide — only one cell at a time into empty space.
- Get the red general's top-left corner to the green outlined slot.
- Match the par for three stars; the legendary Huarong board takes
  116 minimum slides.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — board / pieces model, slide rules, canonical state-
  key for shape-grouped BFS, undo stack, hand-designed levels with
  BFS-verified pars.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, wood frame, per-shape colour ramp, target
  general with centre glyph, selection ring, HUD, stars.
- `js/game.js` — screen flow, tap-select + drag-to-slide + keyboard,
  save.
