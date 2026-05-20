# Pixel Vector

A pixel-art **Asteroids**-style space shooter. Drift, rotate, thrust, and fire
to blast the asteroid field — the edges wrap, so you can slip right off one
side and reappear on the other. A fresh free-flight shooter genre alongside
the other `samples/` pixel games.

## Features

- 8-sector campaign with rising asteroid counts and a few sectors that start
  with medium or pre-broken small asteroids already in the mix — the final
  two sectors (Tempest and Singularity) seed in 4 and 8 small rocks
  respectively for a frantic close.
- Big asteroids split into two mediums, mediums split into smalls; clear them
  all to win the sector.
- Inertial flight with a hard speed cap; light drag bleeds momentum so the
  ship stays controllable.
- On-screen ROTATE / THRUST / FIRE buttons work cleanly on mobile and desktop.
- Three lives, respawn invulnerability blink, score for every kind of rock,
  per-sector best scores, progressive unlocks — all saved to `localStorage`.
- Every one of the eight sectors verified clearable by an automated aim-
  and-fire bot.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4256
```

Then visit `http://127.0.0.1:4256/index.html`.

## Play

- Hold ROTATE LEFT / RIGHT to turn the ship.
- Hold THRUST to accelerate in the direction you're facing.
- Tap or hold FIRE to shoot.
- Asteroids split when hit — chase the pieces down. Clear the field to win.

## Structure

- `index.html` - shell, title / sector-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - ship physics, asteroids, bullets, collision.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, ship, asteroids, bullets, on-screen controls.
- `js/game.js` - real-time loop, multi-touch input, save.
