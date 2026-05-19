# Pixel Auto Arena

A dependency-free auto-battler — draft a squad of pixel creatures, merge
duplicates, line them up, and watch them fight escalating enemy squads on their
own. A fresh strategy genre alongside the other `samples/` pixel games.

## Features

- 9 units across 3 classes (Beast / Mech / Mage), each in tiers 1-3 with its own
  pixel sprite.
- Buy three of the same unit and they auto-merge into a starred, far stronger
  version (up to 3 stars).
- Class synergies: field 2 or 4 of a class to buff every unit of that class.
- Positioning matters — the front unit fights first, so arrange your line by
  tapping two units to swap them.
- Auto-resolved line battles with hit flashes, damage popups, and live HP bars.
- Round-by-round economy: gold income, reroll, sell, a win-streak gold bonus,
  and a growing squad capacity.
- 5 lives, endless escalating rounds, and a `localStorage` best-round record.
- English / 中文 toggle.
- Responsive canvas; fully playable with taps on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4195
```

Then visit `http://127.0.0.1:4195/index.html`.

## Play

- Buy units from the shop with gold; three identical units merge automatically.
- Tap a unit on the arena to select it, tap another to swap their order, or
  press **Sell**. **Reroll** for a fresh shop.
- Build class synergies — the bar under the arena shows your counts.
- Press **BATTLE** to fight the round's enemy squad. Lose and you drop a life;
  survive as many rounds as you can.

## Structure

- `index.html` - title / game screens, HUD, shop, overlays.
- `css/style.css` - responsive UI, shop cards, synergy chips.
- `js/i18n.js` - English / Chinese strings and unit names.
- `js/data.js` - classes, unit roster, economy tuning.
- `js/art.js` - the 9 pixel unit sprites and the arena backdrop.
- `js/game.js` - shop, merging, synergy, battle simulation, rounds, save.
