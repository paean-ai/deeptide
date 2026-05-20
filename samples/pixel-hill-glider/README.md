# Pixel Hill Glider

A pixel-art hill-gliding game. Tuck and dive into the downslopes to build
speed, then release at a crest to launch your bird into the sky. Glide as far
as you can — and catch light orbs before dusk falls.

## Features

- Momentum gliding physics: a slope-following glide model where diving on a
  downhill accelerates you and a well-timed crest launches you airborne; a
  misaligned landing bleeds your speed.
- One-button control — hold to tuck and dive, release to glide.
- Light orbs arc above the hills; collecting them refuels your daylight and
  builds a 5-level fever multiplier for bigger scores. Every seventh orb in
  the chain is a **golden orb** — a 3× refuel and a 5× score chip, marked
  by a brighter halo and four pixel sparkles.
- A draining light meter ends the run at dusk — keep catching orbs to glide on.
- Parallax hills, a sinking sun and a sky that warms from day to dusk as your
  light fades.
- `localStorage` best-distance record.
- English / 中文 toggle.
- Responsive 480:300 canvas — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4208
```

Then visit `http://127.0.0.1:4208/index.html`.

## Play

- Hold anywhere (or `Space`) to tuck — gravity pulls harder, so you dive.
- Dive *down* the slopes to gather speed; release as you reach a crest to fly.
- Fly through light orbs to refuel daylight and raise your fever multiplier.
- Land smoothly along a slope to keep your speed; a hard landing breaks fever.
- When the light runs out, dusk falls and the run ends.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 480:300 shell, HUD, screens.
- `js/data.js` - physics constants, terrain function, orb placement.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - sky, parallax hills, bird and orb rendering.
- `js/game.js` - run state, glide physics, orbs, scoring, save.
