# Pixel Sky Climber

A dependency-free endless vertical bouncer — steer a springy climber up an
infinite tower of platforms, grab coins and jetpacks, dodge monsters, and climb
as high as you dare. A fresh arcade genre alongside the other `samples/` games.

## Features

- Auto-bounce platforming: the climber jumps on its own — you only steer.
- 4 platform types: solid, sliding, crumbling (fall straight through), and
  spring pads that fling you far higher.
- Jetpack pickups for a soaring burst, coins to collect, and spiked monsters to
  stomp or avoid.
- Difficulty climbs with you — platforms thin out and grow trickier the higher
  you go; the sky darkens into a starfield.
- Squash-and-stretch animation, parallax clouds, horizontal screen wrap.
- `localStorage` best-height record.
- English / 中文 toggle.
- Responsive portrait canvas: keyboard (arrows / A·D) or hold the left / right
  half of the screen on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4196
```

Then visit `http://127.0.0.1:4196/index.html`.

## Play

- **Desktop:** left / right (or A / D) to steer, `Esc` to pause.
- **Mobile:** hold the left or right half of the screen to lean that way.
- You bounce automatically on every solid platform — line up the next ledge.
- Crumbling platforms drop you, so aim for solid ground; spring pads launch you
  sky-high; a jetpack makes you briefly unstoppable.
- Fall off the bottom of the screen and the run ends.

## Structure

- `index.html` - title / game screens and overlays.
- `css/style.css` - responsive arcade UI.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - dimensions, physics, platform tuning.
- `js/art.js` - climber, platforms, pickups, monsters, parallax sky.
- `js/game.js` - bounce physics, procedural generation, screens, loop.
