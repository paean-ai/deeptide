# Pixel Data Serpent

A dependency-free modern take on snake — pilot a data serpent through a neon
grid, devour data nodes, clear sectors, and slip through portals while the
firewalls close in. A fresh arcade genre alongside the other `samples/` games.

## Features

- Classic grid-snake movement with crisp, gradient-shaded pixel segments.
- Sector progression: collect 6 nodes to advance — each sector speeds you up
  and drops two more lethal firewall blocks onto the grid.
- 5 node types: standard, **golden** (big score), **shrink** (trims your tail —
  a lifesaver), **slow-mo** (a breather), and **shield** (a pink stack of up
  to two free pardons — a firewall hit dissolves that wall, a self-bite
  trims four segments behind the head).
- Portal pairs appear from sector 3 — dive in one, emerge from the other.
- Solid walls and firewalls are fatal; so is biting your own tail.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive square canvas: keyboard (arrows / WASD), on-screen d-pad, and
  swipe-to-turn on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4197
```

Then visit `http://127.0.0.1:4197/index.html`.

## Play

- **Desktop:** arrow keys / WASD to turn, `Esc` to pause.
- **Mobile:** swipe on the grid or use the d-pad.
- Eat data nodes to grow and score; collect 6 to clear the sector.
- Watch the firewalls pile up — and remember a shrink node can save a run.
- Portals teleport you across the grid; use them to escape tight spots.

## Structure

- `index.html` - title / game screens and overlays.
- `css/style.css` - responsive arcade UI, d-pad.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - grid size, food kinds, sector tuning.
- `js/art.js` - grid, serpent, food, firewalls, portals.
- `js/game.js` - snake logic, sectors, portals, screens, loop.
