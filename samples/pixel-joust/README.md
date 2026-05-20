# Pixel Joust

A pixel-art take on Williams' arcade **Joust** — flap-fly your ostrich
rider around a wrap-around arena and run rivals through; the higher
lance wins on every contact. A fresh medieval flap-fight alongside the
other `samples/` pixel games.

## Features

- 6-wave campaign **Squires → Black Lance** scaling rider count
  (2 → 7) and rider speed (×1.00 → ×1.80).
- Wrap-around arena: fly off the left edge and reappear on the right,
  classic Joust style. Four hand-placed wooden platforms give brief
  perches between flaps.
- **Lance-height duel**: when two riders cross paths, whichever
  lance tip sits higher (smaller y) wins by a `LANCE_GAP = 3` px
  margin — within that margin both bounce off. Higher rider awards
  +100; lower one is unhorsed and loses a life.
- Killed enemies **drop an egg** that hatches into a fresh buzzard
  after 4.5 s — grab the egg first (+25) or pay the price.
- 240 Hz substep inside the variable-dt RAF tick stops fast cross-
  flights from tunneling through each other.
- Three lives + invuln blink on respawn; the wave clears only when
  every rider AND every egg is gone.
- On-screen three-button strip (← ✦ →) with multi-touch pointer
  tracking PLUS arrow keys / WASD / Space (flap) on desktop.
- Per-wave best score saved to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 35 mechanics checks — every wave spawns the right
  enemy count; arena wrap on both edges; lance-height resolver
  reads "higher beats lower beyond LANCE_GAP", "near-tie bumps";
  flap kicks vy negative; horizontal input wraps; player higher
  kills lower enemy on contact and drops an egg; lower player
  loses a life; wave clears when all enemies + eggs are gone;
  three deaths trigger game-over; finalScore adds 100 per life.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4333
```

Then visit `http://127.0.0.1:4333/index.html`.

## Play

- Tap **✦** (or Space / W / Z) to flap upward. Hold **←** or **→**
  to glide horizontally.
- When you cross a rival, the higher lance wins. Time your flaps to
  rise above each one.
- Scoop up the dropped eggs before they hatch into fresh buzzards.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — gravity + flap physics, wrap-around arena, rider AI,
  lance-height resolver, eggs + hatch timer, lives / win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, sky + ground + platforms, ostrich /
  buzzard sprites with lance, egg sprite, HUD, three-button strip.
- `js/game.js` — screen flow, multi-touch + keyboard input, RAF
  loop with 240 Hz substep, save.
