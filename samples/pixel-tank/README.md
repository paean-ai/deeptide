# Pixel Tank

A pixel-art take on the **Battle City** style top-down PvE tank shooter.
A 16×16-cell battlefield with destructible bricks, indestructible steel
walls, an eagle base to defend, and enemy tanks to wipe out. A fresh
top-down arcade entry alongside the other `samples/` pixel games.

## Features

- 6-stage campaign **Outpost → Stronghold** with hand-built 16×16 wall
  layouts and progressively more enemies (2 → 4) per stage.
- 16×16 grid of 16-px cells. Tanks are 14×14 px and move at 50 px/s
  (enemies at 65%). Bullets travel at 200 px/s with cell-aligned
  collision against walls, the eagle, and tanks.
- Three wall types — **brick** (one-shot to destroy), **steel** (blocks
  bullets but cannot be destroyed), and the **eagle** base (one shot = stage
  loss).
- Per-tank single-bullet rule: only one shell in flight per tank at a time.
- Enemy AI: random direction change every ~1 s (immediately on blocked
  movement) plus a periodic fire timer.
- 3 lives — death sends the player back to their spawn; losing all
  3 or letting the eagle fall ends the stage.
- On-screen four-button D-pad + **FIRE** button with multi-touch hold +
  drag-off-to-release; keyboard fallback: WASD / arrow keys + space.
- Per-stage cleared-state save.
- English / 中文, `localStorage` save.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 28 mechanics checks — fresh-state shape, pre-input idle,
  player movement, fire input, all six levels build with enemies + eagle,
  force-killing every enemy wins, a bullet at the eagle cell loses, brick
  destroyed by a forced bullet, steel survives, and a bullet inside an
  enemy AABB kills it + decrements `enemiesLeft`.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4309
```

Then visit `http://127.0.0.1:4309/index.html`.

## Play

- D-pad moves the tank one cell at a time (cell-aligned auto-snap on the
  perpendicular axis).
- FIRE shoots a single shell in the direction the tank is facing.
- Bricks crumble in one hit; steel walls block but never break.
- Wipe every enemy tank to clear the stage; the eagle base must survive.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — wall grid, tank physics, bullets with brick/steel/eagle
  collision, simple enemy AI, win / lose detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, brick + steel + eagle sprites, tank body +
  turret + treads + barrel, bullets, HUD, on-screen D-pad + FIRE pad.
- `js/game.js` — screen flow, multi-touch + keyboard input,
  requestAnimationFrame loop, save.
