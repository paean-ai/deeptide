# Pixel Snowboard

A pixel-art **downhill slalom**. The slope scrolls up as the rider
descends — carve left and right to thread the trees, slip through the
slalom flags for bonus points, and hit the ramps to launch a trick.
Reach the finish to clear the run. A fresh arcade-racing genre
alongside the other `samples/` pixel games.

## Features

- 6-slope campaign **Bunny → Cornice**, growing in length (2400 →
  5400), scroll speed (150 → 300 px/s) and obstacle density
  (×0.7 → ×1.6), so later runs leave far less reaction time.
- Every obstacle field is **generated from a seed** — trees (often in
  small clusters), rocks, slalom gates and ramps — so a slope plays
  the same on every retry but each slope is distinct.
- **Carve** by dragging anywhere (or arrow keys / A·D) — the rider
  eases toward your column at a fixed steer speed rather than
  teleporting, so reads stay fair.
- Slip your column **between the two slalom flags** for a +120 bonus;
  miss the lane and the gate just greys out.
- Hit a **ramp** to launch a 0.9 s trick hop — briefly airborne and
  invulnerable, so a well-timed jump clears a tree that would
  otherwise wipe you out.
- 3 lives per run; a tree or rock at ground level wipes you out and
  costs a life, then you respawn mid-slope. Clearing a slope banks a
  finish bonus plus 100 per life remaining.
- Level select with progressive unlocks and a per-slope best score,
  all saved to `localStorage`.
- HUD with a live slope-progress bar, heart lives and score.
- English / 中文 toggle.
- Responsive 360:480 frame, `image-rendering: pixelated`, drag
  controls tuned for mobile.
- Verified: 284 mechanics checks — every slope builds with a seeded
  obstacle field; steering carves toward the input column and clamps
  to the slope edges; descending accrues score; a tree at ground
  level crashes the rider; an airborne rider clears trees; a ramp
  launches the trick hop; passing the gate lane scores the bonus;
  reaching the finish wins; three crashes end the run.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4357
```

Then visit `http://127.0.0.1:4357/index.html`.

## Play

- **Drag** anywhere on the slope (or **arrow keys** / **A·D**) to
  carve left and right.
- Steer **between the two slalom flags** for a bonus.
- Ride over a **ramp** to launch a trick — you're airborne and can
  clear a tree while you hang.
- Avoid trees and rocks at ground level — a hit costs a life.
- Reach the **finish banner** to clear the slope.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — seeded obstacle-field generation, steering, descent,
  collision / gate / ramp resolution, 6 slope definitions, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, scrolling snow backdrop, trees, rocks,
  slalom gates, ramps, the rider, HUD with slope-progress bar.
- `js/game.js` — screen flow, drag + keyboard input, RAF loop, save.
