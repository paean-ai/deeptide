# Starforge Idle

Dependency-free HTML Canvas pixel idle/incremental sample.

## Features

- Click the **Mine** button *or tap the forge anvil itself* to mine stardust —
  the hammer slams down and throws a burst of sparks on every hit.
- An animated pixel-art star forge: twinkling starfield, glowing star-core,
  swinging hammer, and rising embers whose intensity tracks your auto rate.
- Infinite upgrade levels with exponential costs, each with its own pixel icon.
- Machines for idle production, relics for late-game multipliers, prestige reset.
- LocalStorage autosave and offline earnings.
- English / 中文 toggle.
- Mobile-first controls with large buttons, scrollable shop cards, a tappable
  forge, and safe-area padding; the canvas scales to any screen.

## Run

Open `index.html` directly in a browser, or serve the folder locally:

```bash
python3 -m http.server 4177
```

Then visit `http://127.0.0.1:4177/index.html`.

## Structure

- `index.html` - layout: resource strip, forge, shop tabs, prestige bar.
- `css/style.css` - responsive pixel UI, shop cards, safe-area padding.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - upgrade / machine / relic catalog and cost formulas.
- `js/art.js` - animated forge scene and per-item pixel icons.
- `js/game.js` - production loop, shop, prestige, save / offline.
