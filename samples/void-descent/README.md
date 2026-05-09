# Void Descent Sample

A dependency-free HTML Canvas turn-based dungeon crawler sample.

This sample shares the same pixel-art direction as
`samples/pixel-roguelike`, but expresses a different game type:
room-and-corridor exploration, fog of war, turn-based melee combat,
floor descent, persistent upgrades, and item pickups.

## Run

Open `index.html` directly in a browser, or serve the folder locally:

```bash
python3 -m http.server 4175
```

Then visit:

- Title: `http://127.0.0.1:4175/index.html`
- Game: `http://127.0.0.1:4175/game.html`
- Upgrade selection: `http://127.0.0.1:4175/upgrades.html`

## Structure

- `index.html` - title screen.
- `game.html` - dungeon crawler shell.
- `upgrades.html` - between-floor upgrade selection.
- `css/style.css` - shared pixel UI, responsive layout, panels.
- `js/art.js` - pixel sprite, tile, glyph, and palette helpers.
- `js/dungeon.js` - procedural dungeon generation and entity data.
- `js/game.js` - turn loop, combat, rendering, fog of war, touch input.
- `js/upgrades.js` - upgrade pool, effects, and pixel glyph cards.

This sample intentionally has no package install step and no bundled
`node_modules/`.
