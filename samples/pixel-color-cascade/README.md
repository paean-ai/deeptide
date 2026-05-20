# Pixel Color Cascade

A pixel-art flood-fill puzzle. Your territory starts at the top-left tile —
pick a colour and it spreads, swallowing every connected tile of that colour.
Flood the entire board into one colour before your moves run out.

## Features

- 20 hand-seeded levels on an escalating curve — growing boards (7×7 → 17×17)
  and colour counts (4 → 6).
- Every level's move limit is derived at load time from a built-in greedy
  solver, so each one is guaranteed winnable — and beating the solver earns
  the perfect 3-star rating.
- Ripple-flood animation: tiles flip outward ring by ring from the origin.
- Tap a colour swatch *or* any tile of the colour you want — both flood.
- Level select with progressive unlocks, per-level star ratings and best-move
  records, all saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4207
```

Then visit `http://127.0.0.1:4207/index.html`.

## Play

- Your region is the block of tiles connected to the marked top-left tile.
- Tap a swatch (or a tile) to recolour your whole region to that colour; it
  then merges with any neighbouring tiles already of that colour.
- Make the board a single colour within the move limit. Match par for 3 stars.

## Structure

- `index.html` - shell, title / level-select / game screens, overlays.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - palette, level seeds, board generation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - background, bevelled tiles, board and swatch rendering.
- `js/game.js` - flood-fill, greedy par solver, ripple, scoring, save.
