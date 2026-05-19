# Pixel Glacier

A pixel-art **ice-sliding maze** puzzle. Step in a direction and the explorer
keeps sliding across the ice until a rock or the edge stops them. Route a path
to the glowing exit in as few slides as you can. A fresh sliding-maze genre
alongside the other `samples/` pixel games.

## Features

- 6-level campaign on growing ice fields (6×6 up to 8×8).
- Each floe scatters its rocks from a seed, then a BFS over the slide graph
  **verifies the exit is reachable** and measures the optimal slide count —
  that count is the par you race against.
- 3-star rating by how few slides you use; level select with progressive
  unlocks and per-level star records, all saved to `localStorage`.
- Smooth sliding animation with glossy ice tiles and a pulsing exit portal.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4251
```

Then visit `http://127.0.0.1:4251/index.html`.

## Play

- Tap in a direction from the explorer to send them sliding that way.
- They slide until a rock or the field edge stops them — you cannot stop in
  the middle of the ice.
- Slide onto the glowing exit to escape the floe.
- Reach it in par slides or fewer for the full three stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - level generation and the slide / BFS solver.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, ice, rocks, exit portal, explorer.
- `js/game.js` - sliding play, animation, scoring, save.
