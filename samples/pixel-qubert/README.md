# Pixel Qubert

A pixel-art take on **Q*bert** — hop diagonally around an isometric
pyramid, painting every cube to the target colour while dodging the
balls that bounce down from the apex. A fresh isometric arcade
alongside the other `samples/` pixel games.

## Features

- 7-row pyramid (**28 cubes** total), drawn as proper isometric blocks
  with bevelled side faces and a coloured top diamond.
- 6-level campaign **Aurora → Singularity** scaling the colour chain
  per cube (one-hop → two-hop → three-hop progression) and the enemy
  cadence (4.0 s → 2.0 s spawn, 0.95 s → 0.46 s per descent hop).
- Four diagonal hops (NE / NW / SE / SW); every landing on a cube
  advances its stage by one. Hop off the edge of the pyramid and the
  player tumbles into the void.
- Enemies spawn at the apex and bounce randomly down either south-
  east or south-west each hop; getting caught on the same cube
  costs a life. Three lives, hearts in the HUD.
- Hop arc: a parabolic interpolation between cube-tops so the action
  reads even at the pixel scale; player flashes on respawn.
- Tap any quadrant around the player's cube to hop that way; arrow
  keys / `QEZC` / numpad `7913` work on desktop.
- Level select with progressive unlocks; per-level best score
  persisted to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 67 mechanics checks — pyramid sizes (28 cubes for 7 rows),
  `inBounds` rejects off-edge cells, every diagonal hop produces the
  right (r, c) target, in-bounds hop lands and advances the cube's
  stage, edge hops set `falling` and die on landing, two-hop and
  three-hop levels accumulate the stage exactly, completing the
  final cube triggers the win, three deaths trigger game-over,
  `dirFromTap` maps every quadrant correctly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4325
```

Then visit `http://127.0.0.1:4325/index.html`.

## Play

- Tap a quadrant relative to your cube to hop diagonally up-left,
  up-right, down-left, or down-right.
- Land on every cube the right number of times to fully paint it.
- Avoid the falling red balls — and never hop off the edge.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — pyramid geometry, hop transitions, level stage chains,
  enemy spawn / descent, lives + win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, isometric cube faces, player + enemy sprites
  with parabolic hop arc, HUD.
- `js/game.js` — screen flow, tap-quadrant + keyboard input, RAF, save.
