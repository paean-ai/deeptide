# Pixel Match Quest

A dependency-free HTML Canvas match-3 puzzle with objective levels, obstacles,
boosters, and a level map.

## Features

- Full match-3 engine: swap, match 3+, cascades, gravity, refill.
- Special gems — match-4 makes a line clearer, match-5 a color bomb, and
  L/T shapes a 3x3 bomb; specials chain and activate on swap.
- 5 objective types: reach a score, clear a colour, melt all ice, smash all
  crates, or drop fruit to the bottom.
- 48 hand-built levels across 5 objective types (score, colour, ice, crate,
  fruit-drop) with escalating layouts and difficulty.
- 3 boosters (hammer, shuffle, +5 moves) bought with coins earned from levels.
- Move limits, star ratings, level map with unlocks.
- Auto-shuffle when no moves remain, combo scoring.
- `localStorage` progress save, English/中文 toggle.
- Responsive desktop + mobile: tap-to-select or swipe to swap.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4185
```

Then visit `http://127.0.0.1:4185/index.html`.

## Play

- Swap two adjacent gems to line up three or more of a colour.
- Match 4 or 5, or make an L/T shape, to create powerful special gems.
- Each level has a goal shown on the objective bar — clear it before the
  moves run out.

## Structure

- `index.html` - title / map / game screens and overlays.
- `css/style.css` - responsive pixel UI.
- `js/i18n.js` - English/Chinese strings.
- `js/levels.js` - level layouts, objectives, boosters.
- `js/art.js` - pixel gem, special, ice, crate, and fruit rendering.
- `js/game.js` - match-3 engine, cascades, boosters, screen flow.
