# Pixel Burrow

A pixel-art **whack-a-mole** arcade game. Critters pop from a 3×3 of burrows —
bonk the gophers, grab the golden ones, and never, ever touch a bomb. A fresh
reaction-arcade genre alongside the other `samples/` pixel games.

## Features

- An 8-field campaign that speeds up and turns nastier as it goes.
- Four critter types: gophers score, rare golden gophers score big, bombs
  cost a life if you bonk them, and the new purple **owl** scores 15 ×
  combo AND adds **+5 seconds** to the timer so a fading run can recover.
- A combo multiplier (up to 5×) builds as you land bonks and breaks if you let
  a critter slip away or hit a bomb.
- Beat the score goal before the timer runs out; three lives, three bombs and
  the field is lost. Every field's goal is verified beatable.
- Per-field best scores and progressive unlocks, saved to `localStorage`.
- Critters pop and sink with a clipped emerge animation; floating score popups.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4252
```

Then visit `http://127.0.0.1:4252/index.html`.

## Play

- Tap a gopher the moment it pops up — golden gophers are worth five times as
  much but stay up only briefly.
- Never tap a bomb: it costs one of your three lives.
- Keep bonking without a miss to grow the combo multiplier.
- Reach the score goal before time runs out to clear the field.

## Structure

- `index.html` - shell, title / field-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - field configs, critter spawning and the scoring logic.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, burrows, popping critters.
- `js/game.js` - real-time loop, input, score popups, save.
