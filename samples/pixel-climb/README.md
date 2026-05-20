# Pixel Climb

A pixel-art tribute to **Donkey Kong** — climb a tower of six red
I-beams, dodging the barrels that roll down each beam, and reach the
heart at the top to rescue the princess. A fresh climb-and-dodge
platformer alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Tutorial → Pinnacle** that tightens the barrel
  cadence (2.8 s → 1.3 s) and bumps barrel speed (70 → 124 px/s).
- Six horizontal red I-beams stacked vertically, with rivets, and
  ladders between each pair so the climb route winds back-and-forth.
- Alternating beam-flow direction so barrels naturally snake down the
  tower (top beam flows right, next flows left, etc.) — and barrels
  fall to the next beam once they roll off either end, picking up
  the new beam's flow direction on landing.
- Player movement: walk left / right on a beam, climb up or down a
  ladder, **jump** to leap a rolling barrel (+5 brave bonus per jump).
- On-screen control strip (←  ↓  ↑  →  ✦) plus arrow keys / WASD /
  Space on desktop; jumps are multi-touch friendly.
- 3 lives + invuln blink on respawn; HUD shows level, hearts, score.
- Per-level best score persisted to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 45 mechanics checks — six beams initialise descending +
  alternating flow direction; every level builds with ≥5 ladders and
  the player parked on the bottom beam; horizontal input moves the
  player; jump kicks vy negative; barrels spawn at the configured
  cadence and roll with the beam's BEAM_DIR; reaching the top beam
  wins; falling off the bottom dies; lives decrement on death.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4329
```

Then visit `http://127.0.0.1:4329/index.html`.

## Play

- Hold ← / → (or the on-screen arrows) to walk along a beam.
- Hold ↑ / ↓ on a ladder column to climb / descend; the player snaps
  to the ladder on grab.
- Tap **✦** (or Space) to jump a rolling barrel.
- Reach the heart on the top beam to clear the level. A barrel hit
  or a fall off the bottom costs a life.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — beam geometry, ladders, player physics + ladder snap,
  barrel spawn + roll-and-fall behaviour, life loss / win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, red I-beam with rivets, yellow ladder with
  rungs, brown barrel with rolling bands, player sprite, HUD, on-
  screen control strip.
- `js/game.js` — screen flow, on-screen + keyboard input (multi-
  touch), RAF loop, save.
