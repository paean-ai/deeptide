# Pixel Nurikabe

A pixel-art take on **Nurikabe** — the Nikoli island/sea logic puzzle. Each
numbered cell is the head of a white "island" of that many cells. Shade
every other cell as "sea". A fresh Nikoli classic alongside the other
`samples/` pixel logic games.

## Features

- 6-puzzle campaign (3 × 5×5 + 3 × 6×6). Every puzzle is procedurally
  generated and **verified** to have exactly one solution by a
  shape-enumeration solver (each island is enumerated as a polyomino;
  combinations are pruned by the orthogonal-touch and sea-validity rules).
- Three classic Nurikabe rules enforced together:
  1. Each island has exactly `size` cells, 4-connected, containing one clue.
  2. Two different islands cannot share an orthogonal edge.
  3. The sea is one connected region with no fully-shaded 2×2 block.
- Live red conflict highlight while you solve:
  * Two different islands touching → both flash red.
  * Any 2×2 sea pool → all four cells flash red.
  * An island that has grown beyond its clue → the whole island flashes red.
- Tap a blank cell to cycle **blank → sea → dot** (the dot marks "this
  cell is definitely part of an island, even though I'm not done with it").
- Time + score = `999 - seconds - 30 × mistakes` on a clean solve.
- English / 中文, `localStorage` save with cleared puzzles and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4298
```

Then visit `http://127.0.0.1:4298/index.html`.

## Play

- Tap any blank cell to cycle:
  * **blank** (undecided)
  * **shaded** (sea)
  * **dotted** (white, part of an island)
- Yellow numbered cells are the clue heads — they're always white and you
  can't tap them.
- Win when every cell is decided and every rule is satisfied.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — solution builder (random island placement that respects
  the orthogonal-touch and sea rules), shape-enumeration uniqueness
  verifier, live-violation helpers, win check.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, grid rendering, clue cells, sea / dot icons,
  conflict tint, HUD.
- `js/game.js` — screen flow, tap input, infer-which-island propagation
  for win-check, timer, save.
