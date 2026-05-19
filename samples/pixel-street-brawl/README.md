# Pixel Street Brawl

A dependency-free single-lane beat-'em-up — punch, kick, and combo your way
through escalating waves of street thugs, with a boss every fifth wave. A fresh
brawler genre alongside the other `samples/` pixel games.

## Features

- Punch / kick / jump combat with a 3-hit punch combo that lands a heavy
  finisher.
- 3 enemy types — thugs, brutes, and a hulking boss — with telegraphed attacks
  and hit flashes.
- Endless escalating waves; a boss arrives every 5th wave; difficulty scales.
- Health drops from beaten foes; knockback, particles, and a banner per wave.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive canvas: keyboard (arrows / A·D move, `J` punch, `K` kick, `L`
  jump) or on-screen move pad + action buttons on touch.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4204
```

Then visit `http://127.0.0.1:4204/index.html`.

## Play

- **Desktop:** arrows / A·D to move, `J` punch, `K` kick, `L` / `Up` jump,
  `Esc` to pause.
- **Mobile:** the move pad plus PUNCH / KICK / JUMP buttons.
- Chain three punches for a combo finisher; kicks knock foes back hard.
- Watch enemy wind-ups and step out of reach. Grab dropped hearts to heal.

## Structure

- `index.html` - title / game screens, HUD, controls, overlays.
- `css/style.css` - responsive brawler UI, move pad, action buttons.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - dimensions, fighter stats, wave tuning.
- `js/art.js` - posed fighter sprites and the street backdrop.
- `js/game.js` - combat, enemy AI, waves, screens, save, loop.
