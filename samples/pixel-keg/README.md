# Pixel Keg

A pixel-art **Bomberman-style** dungeon. Drop powder kegs, blast through the
bricks, fend off the wandering monsters, and find the stairs hidden under one
of them. A fresh bomb-and-grid genre alongside the other `samples/` pixel games.

## Features

- 6-floor campaign that grows the maze and adds tougher waves of enemies.
- Real-time grid play on an 11×11 floor with the classic Bomberman pillar
  layout, scattered destructible bricks, and a hidden staircase under one of
  them.
- A powder keg's fuse runs out in two seconds and bursts in a cross of flame —
  longer reach later in the dungeon. Caught in your own blast and you lose a
  life.
- Wandering enemies kill on contact but go up in your flames too.
- On-screen D-pad + BOMB button work cleanly on mobile and desktop.
- Three lives, per-floor completion marks, progressive unlocks, English/中文,
  all saved to `localStorage`.
- Verified: 123 logic checks (bomb cross & blocking, brick destruction, exit
  reveal, flame damage, lives, chain-react bombs) + 4 UI-flow checks (arrow &
  BOMB taps, win on exit).

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4254
```

Then visit `http://127.0.0.1:4254/index.html`.

## Play

- Tap an arrow on the D-pad to step in that direction; hold for steady steps.
- Tap BOMB to drop a powder keg under your feet.
- Stand clear when the fuse runs out — flames kill enemies AND you.
- Find the glowing staircase revealed by a blasted brick to clear the floor.

## Structure

- `index.html` - shell, title / floor-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - floor generation, bomb / flame / enemy rules.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, dungeon tiles, bombs, flames, on-screen controls.
- `js/game.js` - real-time loop, held-button steps, save.
