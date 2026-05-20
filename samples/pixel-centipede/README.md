# Pixel Centipede

A pixel-art **Centipede** — a snaking worm zigzags down from the top of
the screen through a mushroom field; your cannon at the bottom shoots
straight up. Hit any segment and it turns into a mushroom — the worm
splits at that point into two independent worms. A fresh segmented-
enemy arcade alongside the other `samples/` pixel games.

## Features

- 6-wave campaign **Sprouts → Wildwood** with rising centipede length
  (8 → 14 segments) and step speed (×3.5 → ×6.6 cells / sec).
- A 18 × 22 cell board (cell 20 px). The bottom six rows form the
  player zone where the cannon roams; the centipede zigzags through
  rows 0–15 amongst a generated mushroom field whose density rises
  per wave (6 % → 12 %).
- Classic mechanics: each segment moves cell-by-cell, drops one row +
  reverses direction whenever it hits a wall or a mushroom. A shot
  segment becomes a new mushroom and the worm naturally splits — the
  remaining segments inherit their own direction state.
- Mushrooms take 3 hits to clear (cap colour drains red → orange →
  yellow). Shooting them through clears 1 point each.
- A purple spider periodically bounces diagonally through the player
  zone — pick it off for 200 / 300 / 600 (further → closer = higher).
- Autofire so input stays one-finger: drag anywhere to steer the
  cannon (clamped to its bottom six rows); arrow keys / WASD on desktop.
- 240 Hz substep inside the variable-dt RAF tick keeps bullets from
  tunneling through fast segments.
- 3 lives + invuln blink on respawn; HUD shows wave, lives as hearts,
  and the live score. Per-wave best score saved to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 22 mechanics checks — every wave builds with the right
  segment count and an empty player zone, bullet vs mushroom drains
  HP, bullet vs segment removes that segment and stamps a mushroom,
  centipede bounces off walls and mushrooms with drop + reverse,
  player can only sit in the bottom six rows, contact kills, lives
  deplete to game-over, wave clears when no segments remain.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4321
```

Then visit `http://127.0.0.1:4321/index.html`.

## Play

- Drag (or use arrow keys / WASD) to roam your cannon along the bottom
  six rows. Bullets fire automatically.
- Every segment you hit becomes a mushroom — the worm splits there.
- Clear every segment to advance. Shoot mushrooms to thin the field
  and let the worm fall faster. Spider drops are big bonuses.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — grid + waves, mushroom field generation, segment
  step rules, bullet collisions, spider AI, lives.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, mushrooms, segments (head vs body), spider,
  cannon, HUD.
- `js/game.js` — screen flow, drag-to-steer input, RAF loop with
  240 Hz substep, save.
