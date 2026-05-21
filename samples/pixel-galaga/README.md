# Pixel Galaga

A pixel-art take on Namco's **Galaga** — enemies enter from the void in
arcing formations, settle into a slowly drifting grid, then break
formation one at a time to dive-bomb the player. A fresh formation-
shooter alongside the bullet-hell `pixel-bullet-storm` and the vertical
scroller `pixel-sky-raiders`.

## Features

- 9-wave campaign **Recon → Singularity** scaling formation size
  (5×3 → 8×4) and dive cadence (2.6 s → 0.60 s) plus the chance a
  diving enemy fires a bullet on the way down (35 % → 72 %). The
  closing trio — Armada, Tempest, Singularity — dive almost without
  pause.
- Three enemy AI states wired into one update loop:
  **enter** — arc-with-sine from off-screen into the assigned grid
  slot (staggered start so the formation builds up rhythmically);
  **formation** — track the grid origin's gentle left/right drift;
  **dive** — parabolic swoop at the player's last known X, fire
  bullets at the configured chance, then loop back to the slot.
- Player at fixed Y near the bottom; cannon **auto-fires** every
  0.25 s. A formation kill scores 50; a diving kill scores 200.
- 240 Hz substep inside the variable-dt RAF tick stops fast bullets
  from tunneling through small enemy hitboxes.
- Three lives + invuln blink on respawn; HUD shows hearts.
- Drag (or arrow keys / WASD) to slide the fighter; the inputX is
  pursued smoothly so quick swipes feel responsive.
- Parallax starfield with two speed tiers.
- Per-wave best score persisted to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 47 checks — all 9 waves build, dive cadence only
  shortens while swarm and fire-chance only rise across the campaign
  and stay in range; plus a load check that all four scripts run
  cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4335
```

Then visit `http://127.0.0.1:4335/index.html`.

## Play

- Drag (or use ← / →) to slide your fighter; the cannon fires on its
  own cooldown.
- Pick off the **formation** for clean 50-point shots; **diving**
  enemies score 200 but they're aiming at you.
- Dodge bullets and the dives themselves. Three lives.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — formation grid + enemy AI (enter / formation / dive),
  bullet collisions, dive cadence, lives + win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, parallax starfield, ship + bug-shaped enemy
  sprites, bullets, HUD.
- `js/game.js` — screen flow, drag + keyboard input, RAF loop with
  240 Hz substep, save.
