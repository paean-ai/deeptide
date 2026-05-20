# Pixel Archery

A pixel-art **archery** arcade. Drag away from the bow to set aim and
power; release to loose. The arrow flies in a real arc under gravity,
pushed sideways by the level's wind, and scores by the concentric ring
it lands on. A fresh projectile-trajectory arcade alongside the other
`samples/` pixel games.

## Features

- 6-range campaign — **Calm Field → Storm Range** — each range varies
  target size (32 → 22 px outer radius), target distance (300 → 330 px
  out from the archer), and a horizontal **wind** acceleration on the
  arrow in flight (0 → 80 px/s²). The last two ranges roll a *fresh
  random wind every arrow* (`'shift'` pattern), so you can't just
  memorise a single trajectory.
- Real ballistic arc — gravity `480 px/s²`, drag-away slingshot aim
  with a `MAX_POWER` clamp at 720 px/s, and 240 Hz physics substepping
  inside the variable-dt tick so fast shots don't tunnel through the
  target.
- Drag-away slingshot input: pull-back trail behind the bow plus a
  brighter dotted preview *ahead* of the bow showing the arrow's
  launch direction. The end dot turns red as you near max power. The
  bow string visibly pulls toward the drag direction.
- Four-ring scoring: **bull 10 · inner 8 · mid 5 · outer 3**. 10 arrows
  per range; total score chases the best record.
- A wind indicator in the corner shows direction and magnitude; the
  arrow head readouts curve with the throw.
- Past-shot arrows stay stuck on the target board for the rest of the
  round, so you can visually correct your aim.
- English / 中文 toggle, `localStorage` save with per-range best score.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 30 mechanics checks — tiny drag rejected, forward-drag
  (vx ≤ 0) rejected, valid slingshot fires, gravity drops the arrow,
  wind changes landing x, all 10 arrows consume the quiver and end
  the round, *forced bullseye* scores 10 and *forced outer rim*
  scores 3.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4305
```

Then visit `http://127.0.0.1:4305/index.html`.

## Play

- Drag **away** from the bow — pull-back trail behind the bow shows
  power; brighter dotted line in the opposite direction previews where
  the arrow will fly. Release to loose.
- The arrow arcs under gravity and is nudged sideways by the wind.
- Each ring scores a different value; chase the bullseye.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — bow / arrow physics (gravity + wind + 240 Hz substep),
  six ranges, ring math, per-arrow wind shifting, round-end detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, sky + grass + target rings on a wood stand,
  pixel bow with a pull-back string, arrow sprite, slingshot aim guide,
  wind icon, HUD.
- `js/game.js` — screen flow, drag input, requestAnimationFrame loop,
  save.
