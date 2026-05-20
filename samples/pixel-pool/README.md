# Pixel Pool

A pixel-art mini billiards arcade — a 312×312 felt table with four corner
pockets, a white cue ball and a small rack of coloured balls. Drag away
from the cue to aim and set power; release to take a stroke. Clear the
rack before you run out of strokes. A fresh physics-based arcade entry
alongside the other `samples/` pixel games.

## Features

- 6-rack campaign **Solo → Rack**: a single ball up to a full 6-ball
  triangle, with the stroke budget tightening as the rack grows.
- Real ball physics — friction, table-rail bounce (with energy loss),
  and elastic equal-mass ball-to-ball collisions; the velocity exchange
  along the contact normal kicks the impacted ball away and pushes the
  cue off-line.
- 240 Hz physics substepping inside the variable-dt tick keeps fast
  shots stable; the stroke ends the frame every ball is below
  `MIN_SPEED`.
- Drag-AWAY slingshot aim: a dotted trail behind the cue shows your
  pull-back direction, and a brighter dotted line in front previews
  where the ball will roll. The end dot turns red as you near max power.
- Scratch handling: cue-ball-in-pocket = `+1 foul`, cue respawns at the
  break point, and the stroke still counts.
- Score = `100 × balls pocketed + 50 × strokes left − 30 × fouls`. Per
  level best score saved.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 27 mechanics checks — slingshot direction (drag down →
  cue shoots up), physics terminates, wall bounce keeps cue inside the
  rail, cue scratch counts as foul + respawns at break, ball-ball
  contact moves or pockets the colour, all colour balls pocketed
  triggers a win, running out of strokes loses, every level builds with
  the correct ball count.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4303
```

Then visit `http://127.0.0.1:4303/index.html`.

## Play

- Drag away from the cue ball — the trail shows your pull-back, the
  brighter line ahead shows the shot direction; release to fire.
- Pocket every colored ball before the stroke counter runs out.
- Cue ball in a pocket = scratch (+1 foul, ball returns to break).

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — table physics (friction + rail bounce), ball-ball
  elastic collisions, pocket detection, scratch + win / lose logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, table rails + felt, pockets, coloured ball
  sprites with shadow + sheen, slingshot aim guide, HUD.
- `js/game.js` — screen flow, drag input, requestAnimationFrame loop,
  save.
