# Pixel Merge Garden

A dependency-free merge-idle garden — drag matching crops together to grow a
nine-tier crop ladder, fill orders, and bank passive coin income.

## Features

- Drag-to-merge **or** tap-to-merge on a 5x5 plot, with a lifted drag ghost and
  live merge-target highlighting.
- 9 hand-drawn pixel crops, each with a distinct silhouette; palettes loop with
  a prestige gem ring every tier cycle.
- 4 mutation grades (plain / silver / gold / rainbow) that multiply value, plus
  rare **wild crops** that merge with anything.
- Merge combos: chained merges raise a streak multiplier and mutation odds.
- Order board, rain boost, and 8 purchasable greenhouse tiers that scale income
  and seed luck.
- `localStorage` save with capped **offline income** — the garden keeps banking
  coins for up to 8 hours while you're away.
- Juicy particle + floating-text layer rendered on a `requestAnimationFrame`
  overlay canvas.
- English / 中文 toggle, responsive desktop + mobile layout, safe-area aware.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4179
```

Then visit `http://127.0.0.1:4179/index.html`.

## Play

- Drag a crop onto a matching crop (same level) to merge into the next tier — or
  tap one crop then another.
- Wild crops merge with any crop regardless of level.
- Buy seeds to fill empty plots; fill the order to earn bonus coins.
- Collect banked income, trigger rain to double output, and upgrade the
  greenhouse for permanent income and seed-luck gains.

## Structure

- `index.html` - layout: header stats, board, action footer.
- `css/style.css` - responsive pixel UI, crop sway, drag ghost, particle layer.
- `js/data.js` - crops, mutations, greenhouse tiers, board size.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - 24x24 pixel crop sprites and the cell painter.
- `js/game.js` - economy, merge logic, save/offline, particles, input.
