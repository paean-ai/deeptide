# Pixel Fruit Slash

A pixel-art swipe-to-slice action game. Fruit is flung up across the screen —
drag your blade through it to cut it apart, chain combos, and never let your
blade touch a bomb. A fresh slicing-arcade genre alongside the other `samples/`
pixel games.

## Features

- Swipe slicing with real geometry — every blade stroke is tested as a line
  segment against each fruit, so fast flicks cut clean.
- Six fruit types (apple / lemon / melon / berry / orange / dragonfruit)
  that split into two spinning, juice-dripping halves when cut, plus a
  rare golden fruit that pulses with a halo and pays 3× points on a clean
  slice, plus a glowing tapered blade trail.
- Combo scoring — slice three or more fruit in a single swipe for an escalating
  bonus.
- Bombs mixed into the spawns: slice one and the run ends instantly.
- Three lives — let three fruit fall past unsliced and the run is over.
- Endless escalating spawns: bigger waves, faster fruit and more bombs the
  longer you survive.
- `localStorage` best score, particle juice, screen-shake-free pixel feedback.
- English / 中文 toggle.
- Responsive 360:480 playfield — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4226
```

Then visit `http://127.0.0.1:4226/index.html`.

## Play

- Drag across a fruit to slice it — works with a mouse or a finger.
- Cut several fruit in one continuous swipe to land a combo bonus.
- Avoid bombs entirely; one slice ends the run.
- Don't let fruit drop off the bottom — three misses ends the run.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - fruit types and spawn pacing.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, fruit / bomb / half / blade-trail rendering.
- `js/game.js` - swipe slicing, spawning, combos, lives, scoring, save.
