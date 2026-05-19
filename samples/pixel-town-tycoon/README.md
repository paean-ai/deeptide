# Pixel Town Tycoon

A dependency-free HTML Canvas grid management sim — place buildings, wire up
supply chains, and grow a town.

## Features

- 14 building types across producers, processors, homes, a market, and storage.
- Four real supply chains: raw resources → processed goods → coins
  (wheat → flour → bread; wood → plank; cotton → cloth → garment;
  ore + plank → tools).
- Adjacency bonuses (+15%) reward thoughtful placement.
- Worker economy — cottages house workers; under-staffed towns slow down.
- 5 building levels with upgrade costs; demolish for a partial refund.
- 5 town ranks that unlock new buildings; 15 chained quests with rewards.
- Storage caps raised by warehouses; offline production credited on return.
- `localStorage` autosave, English/中文 toggle.
- Responsive desktop + mobile: tap-to-place, tap-to-upgrade, scalable grid.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4184
```

Then visit `http://127.0.0.1:4184/index.html`.

## Play

- Tap a building in the palette, then tap a grass tile to place it.
- Build **cottages** for workers, **producers** for raw goods, **processors**
  to refine them, and a **market** to sell processed goods for coins.
- Place buildings next to their adjacency partner for a production bonus.
- Tap a placed building to upgrade or demolish it. Clear quests to grow.

## Structure

- `index.html` - title / game screens, HUD, palette, overlays.
- `css/style.css` - responsive pixel UI.
- `js/i18n.js` - English/Chinese strings.
- `js/data.js` - buildings, resources, ranks, quests.
- `js/art.js` - pixel building and terrain rendering.
- `js/game.js` - production engine, economy, quests, UI, save/offline.
