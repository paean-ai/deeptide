# Canvas Brick Breaker

A polished arcade brick-breaker — paddle control, brick health, armored bricks,
power cores, multi-ball, laser mode, slow-time, combo scoring, particles,
screen shake, and endless stages.

## Features

- Paddle play with mouse, drag, touch, or arrow keys; Space launches the ball.
- 4 power cores: Wide Paddle, Multi Ball, Laser Paddle, Slow Time.
- Brick health, armored bricks, combo-scaled scoring, particles & screen shake.
- Endless stages that add bricks and density.
- `localStorage` best-score record that survives restarts and reloads.
- English / 中文 toggle.
- Responsive 16:9 canvas — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4184
```

Then visit `http://127.0.0.1:4184/index.html`.

## Structure

- `index.html` - canvas, HUD, hint.
- `css/style.css` - responsive 16:9 shell, HUD.
- `js/i18n.js` - English / Chinese strings.
- `js/game.js` - paddle, bricks, power cores, stages, scoring, save.
