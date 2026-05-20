# Pixel Frogger

A pixel-art **Frogger** — hop across five lanes of traffic and a four-lane
river to fill all five goal pads at the top of the screen. A fresh
grid-arcade crossing genre alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Pond → Maelstrom** with rising traffic speed
  (×1.00 → ×2.00) and tightening time limits (60 s → 36 s).
- A 9-column × 13-row board: start row at the bottom, five road lanes,
  a green median, four river lanes (logs + turtles), bank, and five
  goal pads at the top.
- Real lane logic: cars in alternating directions across five lanes,
  logs (3- and 4-cell) and double-turtle rafts on the river. Stand on
  bare water and you drown; ride the carrier off-screen and you drown.
- **Drag-AWAY-free input** — tap any screen edge (relative to your
  frog) to hop that way, or swipe one-cell-per-32-px. Keyboard arrows
  /WASD work on desktop. Tap dead zone keeps a centred tap from
  registering as a hop.
- 240 Hz substep inside the variable-dt RAF tick so a fast car can't
  tunnel through the frog cell at high speeds.
- Goal pads score 100 + remaining-time bonus + per-row distance bonus.
- Lives are tracked as hearts in the HUD; cars, drowning, off-screen
  river drift, or timeout all cost a life. Three lives.
- Level select with progressive unlocks; per-level best score saved
  to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 18 mechanics checks — every level builds with the right
  road + river lane counts, hop bounds are clamped, hopping past the
  best row scores forward bonus, road collision kills, water without
  a carrier drowns, logs carry the frog horizontally, hopping onto a
  goal pad fills it and resets the frog, the win triggers when all
  five pads fill, lives deplete to game-over.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4319
```

Then visit `http://127.0.0.1:4319/index.html`.

## Play

- Tap the edge of the screen (above / below / left / right of the
  frog) to hop one cell, or swipe in the direction you want.
- Logs and turtles carry you across the river — bare blue is fatal.
- Fill every empty goal pad once. Hitting a filled pad or the bank
  between pads costs a life.
- Beat the lane speeds before the timer runs out.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — board grid, lane definitions, level scaling, hop +
  tick + collision + carrier-ride logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, road / median / water / goal-bank tiles,
  pixel cars, trucks, logs, turtles, frog (4-facing + hop lift), HUD
  with hearts.
- `js/game.js` — screen flow, pointer / keyboard input, RAF loop with
  240 Hz substep, save.
