# Pixel Pinball

A pixel-art pinball table. Charge the launch, then work two flippers to keep
the ball off the drain — bouncing it through pop bumpers, slingshots and drop
targets for as long a run as you can.

## Features

- Real ball physics: gravity, energy-losing wall bounces, and a velocity-based
  speed cap with sub-stepping so the ball never tunnels through walls.
- Two pivoting flippers modelled as rotating segments — their angular velocity
  transfers into the ball, so a well-timed flip launches it hard.
- 3 pop bumpers, 2 slingshots and a 4-piece drop-target bank; clearing every
  drop target pays a big bonus and resets the bank.
- Hold-to-charge launcher with a power meter.
- 3 balls per game, drain detection, score and `localStorage` best-score.
- English / 中文 toggle.
- Responsive 320:440 table — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4206
```

Then visit `http://127.0.0.1:4206/index.html`.

## Play

- When a ball is ready, hold (mouse / touch / `Space`) to charge the launcher,
  release to fire it into the table.
- Tap the **left** or **right** half of the table to work that flipper — or use
  `A` / `D` / arrow keys.
- Hit bumpers, slingshots and drop targets to score. Don't let the ball fall
  through the gap between the flippers.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 320:440 shell, HUD, screens.
- `js/data.js` - table geometry and physics constants.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - table, bumper, target, flipper and ball rendering.
- `js/game.js` - launch, flippers, ball physics, collisions, scoring, save.
