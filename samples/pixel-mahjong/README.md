# Pixel Mahjong

A pixel-art Mahjong solitaire. Tiles are stacked in pyramid layers — match
free pairs to peel the whole stack away. A fresh tile-matching genre alongside
the other `samples/` pixel games.

## Features

- 6-layout campaign of stacked pyramids — growing bases and extra layers
  (32 tiles up to a 152-tile dragon hall).
- Every layout is generated to be solvable: tile faces are dealt by simulating
  a full solve, so a clean path always exists from the start.
- True Mahjong free-tile rule — a tile can be taken only when nothing rests on
  top of it and at least one side edge is clear.
- 9 distinct tile faces, each its own shape *and* colour for instant reading.
- **Shuffle** re-deals the remaining tiles into a fresh solvable arrangement if
  you run out of moves; **hint** flashes a matchable pair.
- 3-star rating — clear a layout with no shuffles or hints for the perfect
  score.
- Level select with progressive unlocks, per-layout star and best-time records,
  saved to `localStorage`.
- Layered pixel tiles with depth shading and a clear-burst particle effect.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4228
```

Then visit `http://127.0.0.1:4228/index.html`.

## Play

- A tile is selectable only when it has nothing on top and a free left or right
  edge — those tiles are drawn bright, blocked tiles are dimmed.
- Tap two matching free tiles to remove the pair.
- Stuck? Tap SHUFFLE to re-deal the rest into a solvable layout, or HINT to
  reveal a pair (both cost you stars).
- Clear every tile to win — do it with no help for 3 stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, control bar, level grid.
- `js/data.js` - pyramid layout campaign and tile expansion.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, layered tile bodies, the 9 tile-face symbols.
- `js/game.js` - solvable generation, free-tile matching, shuffle, save.
