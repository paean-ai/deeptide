# Pixel Mine Sweeper

A pixel-art take on the classic logic puzzle. Dig out every safe tile without
detonating a mine — each uncovered number tells you how many mines hide in the
eight tiles around it. A fresh logic-puzzle genre alongside the other `samples/`
pixel games.

## Features

- 9-level campaign on an escalating curve — growing boards (8×8 → 14×14) and
  rising mine density (~15% → ~22%, classic beginner-to-expert range).
- First-dig safety: mines are placed *after* your first tap, and never under it
  or its eight neighbours, so a game is never lost on the opening move.
- Flood reveal on empty tiles, and chord-digging — tap a satisfied number to
  clear all of its un-flagged neighbours at once.
- Mobile-first input: a DIG / FLAG mode toggle, long-press to flag, plus
  right-click to flag on desktop.
- One **scan** per level safely uncovers a hidden mine-free tile when you're
  stuck.
- 3-star rating per level on clear time, level select with progressive
  unlocks, per-level star and best-time records — all saved to `localStorage`.
- 3×5 pixel-font count digits, bevelled tiles, spiked pixel mines and a
  win-burst particle effect.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4222
```

Then visit `http://127.0.0.1:4222/index.html`.

## Play

- Tap a covered tile to dig it. A number is the count of mines touching it; a
  blank tile floods open its safe neighbourhood.
- Long-press (or switch to FLAG mode, or right-click) to flag a suspected mine.
- Tap an already-revealed number whose flags match its count to chord-dig the
  rest of its neighbours in one move.
- Clear every safe tile to win. Beat the par time for a 3-star rating.

## Structure

- `index.html` - shell, title / level-select / game screens, win & lose overlays.
- `css/style.css` - responsive 360:480 shell, HUD, control bar, level grid.
- `js/data.js` - the 9-level campaign definitions.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - palette, pixel-font digits, bevelled tiles, mine & flag sprites.
- `js/game.js` - grid logic, mine placement, flood/chord digging, scoring, save.
