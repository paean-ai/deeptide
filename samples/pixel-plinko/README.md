# Pixel Plinko

A pixel-art **Plinko / Pachinko** arcade. Tap above the board to drop a
ball; it bounces through a triangular peg grid and lands in one of the
scoring slots at the bottom. Beat the board's score target in 10 balls
to clear the round. A fresh tap-only physics game alongside the other
`samples/` pixel games.

## Features

- 6-board campaign **Carnival → Olympus** with growing peg rows
  (9 → 14) and rising slot values (max 100 → 2000) plus a per-board
  score target (250 → 1600) you have to beat with 10 balls.
- Alternating-row triangular peg grid (8 vs 9 pegs per row) that
  spreads balls naturally; pegs are 4-px circles, ball is a 6-px
  circle, collisions resolved with normal-vector overlap correction
  and an asymmetric sideways nudge so a ball never stops perfectly
  on a peg.
- Real physics — `GRAVITY 720 px/s²`, horizontal damping (`vx *=
  0.985^60dt`), bounce coefficient 0.55 on a peg hit; 240 Hz
  substepping inside the variable-dt tick keeps fast drops stable.
- Slot walls at the bottom guide the ball into one of 9 scoring
  slots; values are colour-coded (red = jackpot, green = high, yellow
  = mid, dim = low).
- Tap-only input — tap anywhere above the slot row to drop a ball
  from your tap-x (clamped to the board). One ball in flight at a
  time.
- HUD shows live score / target and a 10-pip ball indicator.
- Per-board best-score save and a clear gate (beating the target
  unlocks the next board).
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 33 mechanics checks — fresh state, drop spawns + clamps
  to board, ball moves & lands somewhere, second-drop-while-flying
  rejected, a full round of 10 balls ends with 10 landings, all 6
  boards build with pegs + slots + a positive target, drop x clamped
  to board left + right edges.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4315
```

Then visit `http://127.0.0.1:4315/index.html`.

## Play

- Tap anywhere above the slot row — the ball drops from your tap-x.
- Pegs deflect the ball; the slot it lands in determines the score.
- Beat the per-board score target in 10 balls to clear the board.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — peg-grid layout, ball physics (gravity + damping +
  peg bounce + sideways nudge), slot-floor scoring, round finish.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, board, pegs, ball with shadow + sheen, slot
  floor with colour-coded values, past-landing trail dots, HUD with
  ball pips.
- `js/game.js` — screen flow, tap-to-drop input,
  requestAnimationFrame loop, save.
