# Pixel Sumo

A pixel-art top-down **sumo** arena. Drag away from your wrestler to
dash; collisions are elastic and the first wrestler off the ring loses
the bout. A fresh push-off PvE arcade alongside the other `samples/`
pixel games.

## Features

- 6-bout campaign **Novice → Yokozuna** with shrinking AI charge
  cooldown (1.8 s → 0.55 s) and rising charge power (280 → 480 px/s).
- Real physics — friction (`vel *= 0.18^dt`), elastic equal-mass ball-
  ball collision with mass-weighted overlap correction, 240 Hz
  substepping inside the variable-dt tick so a fast head-on doesn't
  tunnel.
- Drag-AWAY slingshot input — pull back from your wrestler, release to
  dash in the **opposite** direction. The aim trail behind shows the
  pull-back, the brighter dotted line ahead shows the dash direction;
  the end dot turns red near max power.
- AI rival charges periodically along the line from itself to the
  player, with per-level cooldown + power; collisions impart real
  momentum so a perfectly-timed dash can send the rival flying.
- 6 distinct rivals; clearing a bout unlocks the next.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 15 mechanics checks — both wrestlers alive at start,
  pre-input idle, drag-AWAY dashes the right way, match resolves,
  AI strength scaling holds (Yokozuna ≥ Novice), all 6 levels build,
  force-pushing AI off the ring wins / player off the ring loses, an
  elastic collision pushes a stationary AI.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4317
```

Then visit `http://127.0.0.1:4317/index.html`.

## Play

- Drag away from your red wrestler — the brighter dotted line shows
  the dash direction; the end dot turns red as you near max power.
- Charge into the rival; the bigger your speed, the bigger the push.
- First wrestler off the dohyō loses the bout.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — sumo arena physics (friction + elastic collision +
  off-ring death), drag-AWAY slingshot, AI charge timer.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, sand-dohyō with edge ring + centre cross,
  red player vs blue AI wrestlers with belts, aim guide, HUD.
- `js/game.js` — screen flow, drag input,
  requestAnimationFrame loop, save.
