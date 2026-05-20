# Pixel Marble

A pixel-art **tilt-to-roll** arcade. Drag anywhere on the screen and the
marble accelerates in the drag direction. Bounce off walls, dodge the
black holes, reach the green pad. A fresh tilt-control entry alongside
the other `samples/` pixel games.

## Features

- 6-room campaign **Foyer → Gauntlet** on a 16×16 cell grid (CELL 20),
  with rising hole counts and maze complexity.
- Drag-from-anywhere tilt input: the pointer-down position becomes the
  tilt origin, and the drag vector (clipped to a 60-px radius)
  normalises into a `(tiltX, tiltY)` in `[-1, 1]` that drives
  acceleration. Works equally on phones and desktop.
- Real physics — `TILT_ACC 760 px/s²` while held, friction (`vel *=
  0.7^dt`), terminal speed clamp at 320 px/s. 240 Hz substepping
  inside the variable-dt tick keeps wall hits stable on fast rolls.
- Walls bounce the marble (35 % retained); holes within 7 px of the
  ball centre swallow it; landing on a `GOAL` cell clears the room.
- Cell-aligned wall AABB collision resolved per axis so the marble
  slides along walls naturally.
- A subtle yellow dotted tilt arrow in front of the marble shows the
  current input direction.
- Per-room cleared-state save + a `999 - seconds` best score.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 35 mechanics checks — pre-input idle, tilt-right
  accelerates rightward, wall collisions don't kill, holes kill on
  centre-touch, goal wins, all 6 rooms have a goal + a clear spawn,
  velocity clamps to `MAX_VEL`.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4311
```

Then visit `http://127.0.0.1:4311/index.html`.

## Play

- Drag anywhere on the field — the drag direction is the tilt
  direction; the further you drag, the harder the tilt.
- Walls bounce, holes swallow, the green pad clears the room.
- Friction slows you down, but high speeds carry momentum into tight
  passages.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — six hand-built maze layouts, tilt-acceleration physics
  with 240 Hz substep, cell-aligned wall AABB resolved per axis, hole
  + goal detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, checker-tile floor, beveled walls, holes
  (rim + inner), goal pad, marble with shadow + sheen, tilt arrow,
  HUD.
- `js/game.js` — screen flow, drag-anywhere tilt input,
  requestAnimationFrame loop, save.
