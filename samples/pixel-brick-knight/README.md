# Pixel Brick Knight

A pixel-art breakout roguelite. Smash through dungeon floors of bricks, and
after every floor pick a power to carry deeper — a fresh brick-breaker-with-
progression genre, distinct from the plain `canvas-brick-breaker` sample.

## Features

- Endless descending floors of bricks with rising hit-points; a boss block
  guards every 5th floor.
- Roguelite progression — clear a floor, then choose one of three random
  powers (heavier hits, a wider paddle, split orbs, extra lives, a phasing
  ball, steadier control, gilded gold) that stack for the whole run.
- Real breakout physics — substepped ball motion, angle-off-the-paddle
  control, multi-ball, and an optional pierce mode.
- Multi-hit bricks coloured by remaining strength; a boss with its own
  health bar.
- 3 lives, gold scoring, particle smash effects, `localStorage` best-floor
  record.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4234
```

Then visit `http://127.0.0.1:4234/index.html`.

## Play

- Drag to steer the paddle; tap to launch the ball.
- Bounce the ball into bricks — tough bricks take several hits.
- Clear every brick to descend, then pick one of three powers.
- Lose a ball with none left and you lose a life; out of lives ends the run.
- Push for the deepest floor.

## Structure

- `index.html` - shell, title / game screens, upgrade & game-over overlays.
- `css/style.css` - responsive 360:480 shell, HUD, upgrade cards.
- `js/data.js` - layout constants, floor generation, the upgrade pool.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, bricks, paddle, ball.
- `js/game.js` - breakout physics, floor flow, upgrades, scoring, save.
