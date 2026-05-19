# Canvas Tower Defense

A focused tower defense game — a fixed enemy path, build pads, escalating wave
spawns, tower placement and upgrades, targeting, projectiles, and base lives.

## Features

- 3 tower types: Arrow (fast single-target), Cannon (slow splash), Frost
  (slows enemies in range), each upgradable and sellable.
- 3 enemy types on depth-scaled waves; bonus gold for clearing a wave.
- Build on pads, pick towers with the menu or keys `1/2/3`, `Space` to start.
- `localStorage` best-score record that survives reloads.
- English / 中文 toggle.
- Responsive 16:9 canvas — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4186
```

Then visit `http://127.0.0.1:4186/index.html`.

## Structure

- `index.html` - canvas, HUD, tower menu, log.
- `css/style.css` - responsive 16:9 shell, HUD, tower menu.
- `js/i18n.js` - English / Chinese strings.
- `js/game.js` - path, pads, waves, towers, projectiles, scoring, save.
