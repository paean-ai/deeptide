# Infinite Pixel Backpack

A dependency-free pixel backpack roguelite sample. It combines backpack spatial
planning, item merging, artifact forging, roguelite upgrades, and autonomous
TD-style waves.

The game defaults to English. Use the `中文` toggle in the HUD to switch the UI
to Chinese.

## Run

Open `index.html` directly in a browser, or serve the folder locally:

```bash
python3 -m http.server 4173
```

Then visit:

```text
http://127.0.0.1:4173/samples/pixel-backpack-roguelite/index.html
```

## Play

- Buy items from the shop and fit them into the backpack grid.
- Drag items to move them, select an item to rotate or sell it.
- Use `合成` to merge two matching items of the same tier.
- Use `锻造` on a selected item when it touches a recipe partner.
- Start the wave and watch the backpack build fight automatically.
- After each win, choose an upgrade and continue indefinitely.

Adjacency is the main system: gems, gears, and batteries strengthen neighboring
weapons. Bosses arrive every fifth wave, and the backpack can expand up to
`8 x 7`.

## Structure

- `index.html` - game shell and script loading order.
- `css/style.css` - responsive pixel UI, backpack grid, shop, and HUD.
- `js/data.js` - item catalog, enemies, upgrades, and recipes.
- `js/assets.js` - pixel glyph and battlefield drawing helpers.
- `js/game.js` - backpack interaction, combat loop, rewards, and rendering.
