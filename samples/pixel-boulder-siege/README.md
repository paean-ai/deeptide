# Pixel Boulder Siege

A pixel-art physics siege game. Fling boulders from your catapult, smash
through wood, glass and stone, and topple the towers until every goblin is
squashed. A fresh projectile-physics genre alongside the other `samples/`
pixel games.

## Features

- Real projectile physics — boulders arc under gravity, ricochet off blocks
  and walls, and lose energy with every impact.
- Three block materials with distinct behaviour: glass shatters in one hit,
  wood splinters, stone barely flinches — and bounces a boulder hardest off
  glass.
- Gravity-settling structures: knock out a support and everything above
  collapses straight down, re-exposing (or burying) the goblins.
- Procedurally generated fortresses — every round builds a fresh set of
  towers, and goblins are never sealed under stone so each round stays
  winnable.
- Drag-to-aim slingshot control with a live dotted trajectory preview and a
  power gauge — pull farther for more force.
- Endless rounds with a limited boulder count per fortress; rising goblin
  counts and tougher stone the deeper you push.
- Score with a leftover-boulder clear bonus, `localStorage` best score.
- English / 中文 toggle.
- Responsive 360:480 field — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4224
```

Then visit `http://127.0.0.1:4224/index.html`.

## Play

- Drag toward the fortress to aim — the dotted line previews the boulder's
  arc, and the gauge shows your power. Release to fire.
- Glass is fragile, wood is medium, stone is tough — break a tower's base and
  the rest crashes down.
- Squash every goblin before your boulders run out to clear the round.
- Each leftover boulder is worth bonus score — finish fast for a high score.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 360:480 shell, HUD, control bar.
- `js/data.js` - layout constants, block types, fortress generation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - scenery, cannon, block / goblin / boulder sprites.
- `js/game.js` - projectile physics, block settling, siege round flow, save.
