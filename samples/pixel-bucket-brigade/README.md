# Pixel Bucket Brigade

A pixel-art **Kaboom!**-style catch arcade. A bomber paces the sky
lobbing bombs — slide your stack of buckets to catch every one before
it reaches the rim. Miss a bomb and a bucket is lost, and the panic
clears the sky. A fresh reaction-arcade genre alongside the other
`samples/` pixel games.

## Features

- Endless escalating waves — each wave the bomber paces faster, drops
  more bombs and mixes in tougher kinds.
- Four bomb kinds for real **skill variety**: a steady **normal**
  bomb, a quick-dropping **fast** bomb, a **cluster** bomb that splits
  into two partway down, and a rare **gold** bomb worth a fat bonus.
- Three power-ups rain down to catch: **+bucket** (a spare life), a
  **slow-motion** window that halves the world's speed, and a
  **magnet** that widens your catch rim.
- A miss costs one bucket *and* the blast wipes every falling bomb —
  lose your last bucket and the sky falls.
- One-thumb control: drag anywhere to slide the bucket stack (or use
  ◀ ▶ / A·D on desktop).
- Best score saved to `localStorage`.
- Chunky pixel art — a pacing bomber, fused bombs, a banded bucket
  stack, a starlit sky.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, drag
  controls tuned for touch.
- Verified: 23 checks — a fresh game starts with three buckets on
  wave one; the stack clamps to the arena; an aligned bomb is caught
  and scores while an off bomb is missed; a miss costs a bucket and
  clears the sky; three misses end the run; a cluster bomb splits
  into two; the magnet catches a bomb beyond the normal rim and a
  normal rim does not; each power-up applies its effect and a missed
  power-up is free; a cleared wave advances; the same seed replays
  identically; plus a UI smoke pass with drag input.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4381
```

Then visit `http://127.0.0.1:4381/index.html`.

## Play

- Drag anywhere (or hold ◀ ▶) to slide your bucket stack along the
  ground.
- Catch each bomb as it reaches the rim — a miss costs a bucket and
  wipes the falling bombs.
- Grab power-ups for a spare bucket, slow-motion, or a wider magnet
  rim.
- Survive as many waves as you can.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — bomber, bomb kinds + cluster split, power-ups, the
  catch / miss resolution, wave escalation, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, sky, bomber, fused bombs, the banded bucket
  stack, HUD.
- `js/game.js` — screen flow, drag + key input, RAF loop, save.
