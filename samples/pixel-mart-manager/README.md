# Pixel Mart Manager

A dependency-free pixel shop-management game — gather stock, restock shelves,
ring up customers before their patience runs out, and grow the mart.

## Features

- Run the floor in person: walk to the grove to harvest, to the shelf to
  restock, to the checkout to ring up paying customers.
- Customers queue with a visible patience bar — serve them in time or lose
  reputation on a missed sale.
- 4 upgrade tracks: Shelf capacity, Helper hires (auto-restock), Mart expansion
  (stock cap + rep), and Marketing (faster customers, bigger receipts).
- Delta-timed simulation — runs identically at 60 / 120 / 144 Hz.
- `localStorage` save with **offline helper income** (helpers keep earning for
  up to 8 hours while you're away).
- English / 中文 toggle.
- Crisp pixel-sprite art with a CRT-styled storefront.
- Responsive: scales on desktop, fills the screen with an on-screen d-pad and
  ACT button on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4178
```

Then visit `http://127.0.0.1:4178/index.html`.

## Play

- **Desktop:** arrow keys / WASD to move, `Space` / `Enter` to interact.
- **Mobile:** on-screen d-pad to move, ACT to interact.
- Stand in the **Banana Grove** and act to harvest stock; carry it to the
  **Shelf** and act to restock; meet paying customers at **Checkout** and act.
- Spend coins on upgrades — helpers restock for you, marketing pulls in
  customers faster.

## Structure

- `index.html` - canvas, HUD, upgrade panel, touch controls.
- `css/style.css` - responsive CRT shell, HUD, upgrade buttons, d-pad.
- `js/data.js` - palette, sprites, zones, upgrade cost tuning.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - pixel drawing primitives (sprites, panels, bars).
- `js/game.js` - simulation, customers, upgrades, save/offline, loop.
