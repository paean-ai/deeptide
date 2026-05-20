# Pixel Copter

A pixel-art take on the classic **Helicopter** cave-flyer. Hold the screen
to thrust upward, release to fall, and squeeze through a procedural
scrolling cave that narrows the farther you fly. A fresh
continuous-thrust arcade alongside the other `samples/` pixel games
(distinct from the tap-impulse `pixel-flap`).

## Features

- 6-cave campaign **Bay Pass → Abyss Run**, with scroll speed climbing
  from 110 → 230 px/s and the cave gap shrinking from a per-level start
  width down to a per-level `gapMin` (170 → 88 px) as you fly farther.
- Continuous-thrust physics — gravity `700`, thrust adds `-900` while
  held; terminal fall capped at `±520` px/s. The copter sprite tilts
  with its vertical velocity.
- Procedural cave by a 12-px-sampled polyline that drifts its centre
  every 5 samples (lerp toward a fresh random target) plus a per-level
  shrinking gap. Pillars spawn every `pillarEvery` px and block the
  middle of the cave gap.
- Score = distance travelled. Per-cave best is saved; clearing a
  per-cave **target distance** (1500 + 500 × levelIndex) unlocks the
  next cave.
- Multi-touch friendly — any pointer or **space / arrow-up / W** key
  counts as thrust; the copter rises as long as any input is held.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 37 mechanics checks — pre-input idle, gravity drop after
  thrust released, thrust raises the copter, world scrolls, ceiling +
  floor crash, every level run terminates under a deterministic
  hold-6/release-6 bot, every level builds with sane config, and
  sample-interpolation edge cases.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4307
```

Then visit `http://127.0.0.1:4307/index.html`.

## Play

- Hold the screen (or **space**) to thrust upward; release to drop.
- Squeeze through the cave and dodge the pillars.
- Beat the per-cave target distance to unlock the next cave.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — copter physics, procedural cave samples + pillar
  spawning, crash detection, distance + win checks.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, gradient sky + parallax stars, cave polygons
  with edge lines, pillars with caps, pixel helicopter with a rotor
  blur + cockpit window, HUD.
- `js/game.js` — screen flow, multi-touch hold input + keyboard
  fallback, requestAnimationFrame loop, save.
