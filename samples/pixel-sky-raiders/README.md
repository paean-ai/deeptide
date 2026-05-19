# Pixel Sky Raiders

A dependency-free vertical shoot-'em-up — pilot the raider, auto-fire through
endless enemy waves, dodge bullet patterns, grab power-ups, and break bosses.
A fresh arcade genre alongside the other `samples/` pixel games.

## Features

- Auto-firing raider with 5 escalating weapon tiers (single shot → 5-way spread).
- 4 enemy archetypes — divers, sine weavers, aimed turrets, spread tanks — that
  unlock as the waves climb.
- A boss every 5th wave with a health bar and three cycling bullet patterns.
- Power-ups: weapon up, spare bombs, and a timed shield.
- Screen-clearing bombs that wipe enemy fire and hammer everything on screen.
- Parallax starfield, particle bursts, hit flashes, i-frames after a hit.
- Endless, depth-scaled difficulty and a `localStorage` best-score record.
- English / 中文 toggle.
- Responsive portrait canvas: keyboard (arrows / WASD) on desktop, drag-to-fly
  on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4192
```

Then visit `http://127.0.0.1:4192/index.html`.

## Play

- **Desktop:** arrow keys / WASD to fly, `Space` or `B` to bomb, `Esc` pauses.
- **Mobile:** drag anywhere on the playfield — the raider follows your finger;
  tap the BOMB button to clear the screen.
- Guns fire automatically — focus on positioning and dodging.
- Collect **P** for a weapon upgrade, **B** for a bomb, **S** for a shield.
- Survive the boss waves and chase a higher score.

## Structure

- `index.html` - title / game screens, HUD, overlays.
- `css/style.css` - responsive arcade UI, HUD, overlays.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - dimensions, enemies, weapon tiers, power-ups, wave tuning.
- `js/art.js` - pixel sprites for the raider, enemies, boss, bullets, pickups.
- `js/game.js` - waves, firing, collisions, boss patterns, screens, loop.
