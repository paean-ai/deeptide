# Pixel Road Hop

A pixel-art endless lane-crossing game. Hop your chick forward across busy
roads and rushing rivers — dodge the cars, ride the logs, and don't dawdle.

## Features

- Endless procedurally generated lanes — grass (with trees to route around),
  roads with cross-traffic, and rivers you cross by riding drifting logs.
- Difficulty ramps with distance: faster, denser traffic the further you go.
- A camera that creeps forward when you idle — stop too long and you're caught.
- Three ways to die — flattened on the road, drowned in the river, or left
  behind — each with its own game-over line.
- One-hop-per-input control: swipe in any direction, tap to hop forward, or use
  the arrow / WASD keys.
- `localStorage` best-distance record.
- English / 中文 toggle.
- Responsive 360:480 canvas — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4210
```

Then visit `http://127.0.0.1:4210/index.html`.

## Play

- Swipe (or press a direction key) to hop one tile. A tap hops forward.
- Time your road crossings between cars. On a river you must land on a log —
  open water drowns you, and a log can carry you off the edge.
- Keep moving: the screen slowly scrolls forward and will catch a straggler.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 360:480 shell, HUD, screens.
- `js/data.js` - tile dimensions and camera constants.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - lane, tree, car, log and player rendering.
- `js/game.js` - row generation, traffic, hopping, scoring, save.
