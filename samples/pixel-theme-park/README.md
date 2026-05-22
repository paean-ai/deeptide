# Pixel Theme Park

A dependency-free pixel-art balance rides, queues, snacks, and guest happiness. sample with a complete loop: resources, facilities, upgrades, timed actions, events, save/load, and bilingual UI.

## Features

- Six-resource economy tuned around meaningful tradeoffs rather than one-click accumulation.
- Six unlockable facilities with level scaling, cost curves, and visual map presence.
- Six distinct actions with duration, costs, rewards, XP, and facility synergies.
- Random events tied to the current strategy, including positive and negative pressure.
- Milestone progression, win condition, restart, pause, localStorage autosave.
- English/中文 toggle and responsive desktop/mobile controls.
- Canvas pixel scene with animated workers, facility sprites, particles, and event feedback.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4194
```

Then visit `http://127.0.0.1:4194/index.html`.

## Play

- Choose an action card to queue work. Actions consume resources immediately and complete after their timer.
- Upgrade facilities to improve action rewards and unlock stronger pacing.
- Watch the goal meter. Reaching **Joy 85** wins the run.

## Structure

- `index.html` - app shell and HUD.
- `css/style.css` - responsive pixel UI and panels.
- `js/i18n.js` - bilingual strings.
- `js/data.js` - game-specific resources, actions, events, upgrades.
- `js/art.js` - pixel drawing helpers.
- `js/game.js` - loop, economy, save, UI, input, and canvas rendering.

