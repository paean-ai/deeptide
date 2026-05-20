# Pixel Hop

A pixel-art **vertical climber** — drag to steer, auto-jump on every platform,
chase the altitude target before you drop off the screen. A fresh take on the
Doodle-Jump-style genre that sits alongside the side-scroller
(`pixel-platformer-infinite`) and the side-view flapper (`pixel-flap`) in
`samples/`.

## Features

- 6-level campaign — Foothills 900m, Cliffs 1600m, Spires 2400m, Skyway 3400m,
  Stratos 4600m, Apex 6200m. Each route is a deterministic seed so a level is
  the same layout every time.
- Four platform types for skill variety:
  - **Plank** — regular bounce.
  - **Mover** — slides left and right within a band; time the landing.
  - **Spring** — launches you ~1.6× higher; useful for crossing wide gaps.
  - **Cloud** — vanishes after one bounce; commit to the next platform fast.
- Gems sprinkled along the climb add +50 to the post-clear score.
- Doodle-Jump-style horizontal wrap, parallax star field, camera follows the
  highest point so falling back is dangerous.
- English / 中文, `localStorage` for cleared routes and best scores, six-cell
  level grid with locked levels.
- Verified: 28 mechanics checks (rise after flap, tilt, wrap, spring +
  cloud + mover platforms, gem pickup, win at target, lose below screen) +
  14 live-system checks (every level builds, runs under a scripted bot, and
  the bot makes upward progress).

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4280
```

Then visit `http://127.0.0.1:4280/index.html`.

## Play

- Tap to launch your first jump, then **drag anywhere** on the field — the
  player tilts toward the touch point.
- Keyboard fallback: **arrow keys** or **A/D** steer; **space** to launch.
- You bounce automatically on every platform you land on. Spring platforms
  launch you harder; cloud platforms vanish after one use; movers slide.
- Reach the altitude target shown in the HUD to clear the route. Drop below
  the bottom of the screen and you fall.

## Structure

- `index.html` — shell, title / level / play / result screens.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — physics, level configs, platform spawning, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — sky gradient, parallax stars, platforms, gems, player sprite.
- `js/game.js` — screen flow, drag input, requestAnimationFrame loop, save.
