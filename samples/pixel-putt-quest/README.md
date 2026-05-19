# Pixel Putt Quest

A pixel-art mini-golf game. Drag back from the ball like a slingshot to aim and
set power, then release to putt. Bank off walls, dodge water, escape the sand,
and sink the ball under par across a 9-hole course.

## Features

- Slingshot aiming: drag distance sets power (with a power ring + dotted
  trajectory guide), drag direction sets the launch line.
- Full ball physics — wall bounces with energy loss, grass vs. sand friction,
  water hazards that splash the ball back with a one-stroke penalty, and a cup
  that captures slow balls near the rim.
- 9 hand-built holes (par 2–5) with walls, sand bunkers and water, on an
  escalating difficulty curve.
- Hole select with progressive unlocks and per-hole best-stroke records.
- `localStorage` save: unlocked holes, best strokes per hole, best round total.
- English / 中文 toggle.
- Responsive canvas — scales on desktop, fills the screen on mobile; touch drag
  aiming works the same as mouse.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4205
```

Then visit `http://127.0.0.1:4205/index.html`.

## Play

- Drag from anywhere toward the ball's *opposite* side — the ball flies away
  from the drag, slingshot-style. Longer drag = more power.
- Release to putt. Wait for the ball to stop before the next shot.
- Sink the ball in the cup. Water costs +1 stroke; sand kills your roll.
- Beat par on every hole; finish all 9 for your round total.

## Structure

- `index.html` - shell, title / hole-select / game screens, overlays.
- `css/style.css` - responsive 480:340 shell, screens, HUD, hole grid.
- `js/data.js` - course dimensions and the 9 hole definitions.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - grass, terrain, walls, cup, flag, ball and aim rendering.
- `js/game.js` - aiming, ball physics, hole flow, scoring, save.
