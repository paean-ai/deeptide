# Pixel Vanguard

A pixel-art **turn-based mech tactics** game in the spirit of Into the Breach.
Every enemy telegraphs the tile it will strike next turn — move your two mechs,
hit hard, and use the knockback to shove threats off your buildings. A fresh
tactics genre alongside the other `samples/` pixel games.

## Features

- An 8-mission campaign on a 6×6 grid, rising in enemy count and battlefield
  clutter — the final two add a four-wall Crucible and a six-enemy Rampart
  finale with a 14-HP core.
- Enemies show a red danger tile each turn — exactly where they will strike.
  Read it, then act before it lands.
- Every mech attack **pushes** its target back one tile: shove an enemy off a
  building, into a wall for collision damage, or into another enemy.
- A shared core powers your buildings; lose it and the grid goes down. Clear
  every enemy to secure the area.
- All eight missions are verified winnable by an automated look-ahead bot.
- Level select with progressive unlocks and per-mission completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4248
```

Then visit `http://127.0.0.1:4248/index.html`.

## Play

- Tap a mech to select it, tap a green tile to move, then tap an adjacent
  enemy to strike — the hit knocks the enemy back a tile.
- Red tiles mark where enemies will hit on their turn. Push or kill them so
  nothing lands on your buildings.
- Press END TURN to let the enemies act, then plan the next round.
- Wipe out every enemy to win; if the core hits zero, the mission is lost.

## Structure

- `index.html` - shell, title / mission-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - missions and the turn-based combat rules.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, grid, mechs, enemies, danger telegraphs.
- `js/game.js` - selection / move / attack UI, turn flow, save.
