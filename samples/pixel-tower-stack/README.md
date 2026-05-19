# Pixel Tower Stack

A dependency-free one-tap stacking game — time each sliding block to land it
cleanly on the tower. Misaligned overhang is sliced off; nail it perfectly and
the block keeps its full width. A fresh arcade genre alongside the other
`samples/` pixel games.

## Features

- Pure one-tap play: drop the sliding block — that's the whole control scheme.
- Overhang is sliced away as tumbling debris; the tower narrows as you climb.
- **Perfect drops** (pixel-aligned) keep full width, and a hot perfect-streak
  begins regrowing the block — recover from sloppy stacks.
- Rising camera, speed that ramps with height, a rainbow hue gradient up the
  tower, and a sky that brightens as you ascend.
- Sliced-debris physics, perfect-stack sparkles, combo counter.
- `localStorage` best-height record.
- English / 中文 toggle.
- Responsive portrait canvas — identical play with tap, click, or space.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4198
```

Then visit `http://127.0.0.1:4198/index.html`.

## Play

- Tap / click anywhere (or press space) to drop the moving block.
- Land it over the block below — the non-overlapping part is cut away.
- Aim for **perfect** alignment to keep your width; chain perfects to regrow it.
- Miss completely and the tower topples. Climb as high as you can.

## Structure

- `index.html` - title / game screens and overlays.
- `css/style.css` - responsive arcade UI.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - dimensions, speed and width tuning.
- `js/art.js` - block, debris, sparkle, and sky rendering.
- `js/game.js` - stacking / slicing logic, camera, screens, loop.
