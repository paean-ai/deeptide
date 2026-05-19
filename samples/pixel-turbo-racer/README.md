# Pixel Turbo Racer

A dependency-free endless arcade racer — weave through traffic, chain
near-misses for combo points, grab nitro, and chase a high score. A fresh
arcade genre alongside the other `samples/` pixel games.

## Features

- Endless four-lane highway with smooth difficulty ramp — traffic gets faster
  and denser the longer you survive.
- Near-miss combo system: shave past a car without touching it to build a combo
  multiplier worth ever-bigger point bonuses.
- Nitro boost: collect canisters, then burn one for a speed surge that lets you
  smash straight through traffic.
- Hazards with variety — cones wreck you, oil slicks send you into a spin.
- Coins, parallax roadside trees, particle crashes, scrolling rumble strips.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive portrait canvas: keyboard (arrows / WASD), or drag-to-steer with a
  NITRO button on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4193
```

Then visit `http://127.0.0.1:4193/index.html`.

## Play

- **Desktop:** left / right (or A / D) to steer, `Space` / `Up` for nitro,
  `Esc` to pause.
- **Mobile:** drag anywhere on the road — the car follows your finger; tap the
  NITRO button to boost.
- Slip past cars as closely as you dare — near-misses feed your combo.
- Cones and other cars are fatal; oil slicks just spin you out. Nitro makes you
  briefly unstoppable.

## Structure

- `index.html` - title / game screens, HUD, overlays.
- `css/style.css` - responsive arcade UI, HUD, nitro button.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - dimensions, road layout, traffic, difficulty tuning.
- `js/art.js` - pixel cars, pickups, hazards, roadside props.
- `js/game.js` - driving, spawning, collisions, combo, screens, loop.
