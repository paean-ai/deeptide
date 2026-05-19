# Pixel Orb Fusion

A dependency-free 2048-style merge puzzle — slide every orb across the grid;
equal orbs fuse into the next tier. Forge the 2048 orb, then chase a record.
A fresh puzzle genre alongside the other `samples/` pixel games.

## Features

- Classic 4x4 slide-and-merge rules with crisp tier-coloured pixel orbs.
- Smooth slide animation and a satisfying pop when orbs fuse.
- One-step **undo** — take back a careless swipe.
- Win banner when you forge the 2048 orb; keep playing for a high score.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive square canvas: keyboard (arrows / WASD), on-screen d-pad, and
  swipe-to-slide on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4200
```

Then visit `http://127.0.0.1:4200/index.html`.

## Play

- **Desktop:** arrow keys / WASD to slide, `Z` to undo, `Esc` to pause.
- **Mobile:** swipe in any direction; or use the d-pad.
- Every swipe slides all orbs as far as they can go; two touching orbs of the
  same value fuse into one of the next value.
- A new orb appears after each move — the grid locks when it is full with no
  fusions left.

## Structure

- `index.html` - title / game screens and overlays.
- `css/style.css` - responsive puzzle UI, d-pad.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - grid constants, orb tiers, swipe-line helper.
- `js/art.js` - board and orb rendering.
- `js/game.js` - slide / merge logic, animation, undo, save, screens.
