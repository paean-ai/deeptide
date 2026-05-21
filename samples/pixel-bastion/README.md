# Pixel Bastion

A pixel-art **Missile Command**. Tap anywhere in the sky — the closest silo
with ammo fires a counter-missile that explodes at your target, taking out any
incoming missile caught in the blast. A fresh sky-defence arcade genre
alongside the other `samples/` pixel games.

## Features

- 9-wave campaign that escalates incoming count and speed from a calm First
  Wave to a desperate Armageddon.
- The final three waves — **MIRV Rain, Hydra Sky, Armageddon** — add
  **MIRV missiles**: a pulsing amber warhead that splits into two fresh
  warheads partway down. Catch it above the split altitude and you stop
  both halves at once; miss it and one threat becomes two.
- Three silos with limited ammo per wave; five cities to defend.
- Lead-aim a tap — the counter takes ~1 second to arrive and the blast lasts
  a moment, so plan ahead.
- Per-wave best scores and progressive unlocks, English/中文,
  `localStorage` save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4262
```

Then visit `http://127.0.0.1:4262/index.html`.

## Play

- Tap anywhere above the ground to launch a counter-missile from the silo
  closest to your target.
- The blast destroys any incoming missile inside its radius — chain
  intercepts for big scoring.
- On the later waves, prioritise the amber MIRVs before they split.
- Survive each wave with at least one city standing.

## Structure

- `index.html` - shell, title / wave-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - silos, cities, incoming + counter missiles, MIRV
  splitting, blast logic.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, ground, cities, silos, missile trails, pulsing
  MIRV warheads, blasts.
- `js/game.js` - real-time loop, tap input, save.
