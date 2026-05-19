# Pixel Shadow Heist

A dependency-free turn-based stealth game — sneak a thief past patrolling
guards to the vault. Every move is a turn; the guards step after you. A fresh
stealth genre alongside the other `samples/` pixel games.

## Features

- Turn-based stealth: each step is a turn, then every guard advances one tile
  along its looping patrol — read the rhythm and slip through.
- Guard vision is a straight ray (3 tiles) in the facing direction, blocked by
  walls and shown as a red cone — so you can plan.
- A **wait** move lets a patrol pass; getting spotted snaps you back to the
  start, so experiment freely.
- 8 hand-built levels of rising difficulty, with a level-select grid.
- `localStorage` progress: unlocked levels and best turn counts persist.
- English / 中文 toggle.
- Responsive canvas: keyboard (arrows / WASD), on-screen d-pad with a wait
  button, and swipe-to-move (tap to wait) on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4199
```

Then visit `http://127.0.0.1:4199/index.html`.

## Play

- **Desktop:** arrow keys / WASD to move, `Space` to wait a turn, `R` to restart.
- **Mobile:** swipe to move, tap to wait; or use the d-pad.
- Guards only see straight ahead — cross behind their backs, never in front.
- Reach the glowing vault tile to crack the job. Spotted? You restart the
  level — no penalty but the turn count.

## Structure

- `index.html` - title / level-select / game screens and overlays.
- `css/style.css` - responsive stealth UI, level grid, d-pad.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - level grids, guard patrols, the level parser.
- `js/art.js` - tiles, the thief, guards, vision cones.
- `js/game.js` - turn loop, guard model, vision, save, screen flow.
