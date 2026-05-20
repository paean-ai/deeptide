# Pixel Flap

A pixel-art **flappy** arcade. Tap to flap upward; gravity pulls you back
down. Slip the bird through every gap in the scrolling pipes and pass the
target number to clear the sky. A fresh one-button arcade alongside the
other `samples/` pixel games.

## Features

- 6-sky campaign with rising scroll speed and shrinking gaps.
- One-button play: tap anywhere to flap.
- Wait-for-first-flap start so you can read the first pipe layout before the
  scroll begins.
- Per-sky completion marks and progressive unlocks, English/中文,
  `localStorage` save.
- Verified: 6 physics-and-collision checks (flap velocity, gravity fall,
  ground death, win on target) + an end-to-end run that proves the system
  terminates on every level and a simple heuristic bot can clear at least
  one sky (10+ pipes passed total across the campaign) + a UI input check.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4264
```

Then visit `http://127.0.0.1:4264/index.html`.

## Play

- Tap anywhere on the screen to flap upward.
- Gravity always pulls you down — time your flaps to thread the pipe gaps.
- Pass the target number of pipes shown in the HUD to clear the sky.

## Structure

- `index.html` - shell, title / sky-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - bird physics, pipe spawn and collision logic.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - sky, clouds, pipes, bird sprite.
- `js/game.js` - real-time loop, tap input, save.
