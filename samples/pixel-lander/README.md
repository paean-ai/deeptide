# Pixel Lander

A pixel-art take on **Lunar Lander** — fight gravity with a finite fuel
budget, rotate to set your descent angle, and put the lander down on the
landing pad without crashing. A fresh classic-arcade entry alongside the
other `samples/` pixel games.

## Features

- 6-mission campaign from **Sea of Calm** to **Final Descent** — each
  mission ships a different procedural terrain seed, a different pad
  position and width, a different starting fuel budget, and (from L4 on)
  a constant horizontal **wind** that you have to tilt against.
- Real lander physics — gravity, rotation, fuel-gated directional thrust,
  wind acceleration, terrain polyline collision sampled at the lander's
  three skid points.
- Soft-landing window: descent velocity < **28 px/s**, drift < **18 px/s**,
  tilt within **±14°**, and only the flat landing pad counts.
- HUD watch-lights — fuel bar turns red at <25%; vY / vX / TILT each turn
  red the moment they exceed the soft-land window.
- Three-button on-screen control pad with hold-to-continue input, **swipe
  off** a button to release it. Keyboard fallback: **←/→** rotate,
  **space**/**↑** thrust.
- Score = `max(100, fuel_left * 2 + 200)` on a clean landing.
- English / 中文, `localStorage` save with cleared missions and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 24 mechanics checks (gravity, thrust, rotation, fuel drain,
  off-world death, soft-land win, tilted-land crash, all six pads at the
  right elevation, fuel exhausts under sustained thrust, bot lands ≥1
  level under a PD controller).

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4296
```

Then visit `http://127.0.0.1:4296/index.html`.

## Play

- Press and hold **THRUST** (▲) to push along the lander's current up
  vector. Use **◄ / ►** to rotate.
- Watch the HUD — every red value will crash you if you touch down with it.
- Land softly on the green pad to win. Reach a new mission to unlock it.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — physics, six-level terrain generator, soft-land window,
  win / lose detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — sky gradient + star field, terrain polyline, landing pad
  with corner poles, lander sprite + flame, HUD watch-lights, wind arrow,
  three-button control pad.
- `js/game.js` — screen flow, multi-touch pointer + keyboard input,
  requestAnimationFrame loop, save.
