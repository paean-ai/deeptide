# Pixel Roguelike Sample

A small, dependency-free HTML Canvas roguelike used as a DeepTide showcase.

It demonstrates:

- Pixel-art character sprites defined as editable matrix data.
- Responsive desktop and mobile Canvas layout.
- Touch joystick controls for mobile.
- Procedural pixel terrain with biome detail.
- Skill cards, skill glyphs, combat particles, and short-lived visual effects.
- A local pixel-art preview page for future sprite work.

## Run

Open `index.html` directly in a browser, or serve the folder locally:

```bash
python3 -m http.server 4173
```

Then visit:

- Game: `http://127.0.0.1:4173/index.html`
- Pixel-art preview: `http://127.0.0.1:4173/tools/pixel-art-preview.html`

## Structure

- `index.html` - sample shell and script loading order.
- `css/style.css` - responsive game HUD, panels, and touch controls.
- `js/assets.js` - pixel sprite registry and palettes.
- `js/tilemap.js` - deterministic terrain generation and tile rendering.
- `js/renderer.js` - pixel renderer, sprite animation, combat effects.
- `js/game.js` - game loop, input, waves, combat, UI updates.
- `tools/pixel-art-preview.html` - sprite preview helper.

This sample intentionally has no package install step and no bundled
`node_modules/`.
