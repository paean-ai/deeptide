# Pixel Peg Pop

A pixel-art peg-bouncing game in the Peggle tradition. Aim the launcher, drop
a ball, and watch it ricochet through a field of pegs — clear every orange peg
to win. A fresh bounce-physics genre alongside the other `samples/` pixel
games.

## Features

- Real bounce physics — the ball falls under gravity and reflects off every
  peg (substepped circle collisions) and the side walls.
- Two peg types: blue pegs score points, orange pegs are your targets. Pegs
  light up when struck and clear at the end of the turn, so the ball keeps
  bouncing off them mid-flight.
- Drag-to-aim launcher with a live dotted trajectory preview.
- A roaming catcher bucket along the bottom — drop a ball into it for a free
  extra ball.
- 8 hand-built peg layouts on an escalating curve.
- 3-star rating by balls left over, level select with progressive unlocks,
  per-level star records — all saved to `localStorage`.
- End-of-turn combo bonus for clearing several pegs in one shot, with particle
  pops.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4229
```

Then visit `http://127.0.0.1:4229/index.html`.

## Play

- Drag anywhere to aim the launcher — the dotted line previews the ball's
  arc — and release to fire.
- The ball bounces through the pegs; every peg it touches lights up and is
  cleared when the ball leaves the board.
- Clear all the orange pegs before you run out of balls.
- Land a ball in the moving bucket to earn it back. Finish with balls to
  spare for a 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win & lose overlays.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - physics constants and the 8-level peg-layout campaign.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, pegs, ball, launcher, bucket, aim preview.
- `js/game.js` - aiming, ball physics, peg clearing, scoring, save.
