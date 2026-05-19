# Canvas Match-3 Puzzle

A polished match-3 puzzle — level targets, adjacent swaps, cascading refills,
combo scoring, special gems, power clears, and no-move reshuffling.

## Features

- 8x8 board with swap-to-match, cascade refills, and combo-scaled scoring.
- Special gems from 4+ matches: row / column clears, bombs, and a prism.
- Per-level colour targets with a move limit; clearing advances the level.
- `localStorage` best-score record that survives restarts and reloads.
- English / 中文 toggle.
- Responsive square canvas, playable with mouse or touch.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4185
```

Then visit `http://127.0.0.1:4185/index.html`.

## Structure

- `index.html` - header stats, canvas, footer buttons.
- `css/style.css` - responsive layout, HUD.
- `js/i18n.js` - English / Chinese strings.
- `js/game.js` - board, matching, specials, levels, scoring, save.
