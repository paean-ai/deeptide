# Pixel Leap

A pixel-art **single-screen precision platformer**. Run, jump and
air-dash through a hand-built room of ledges and pits to reach the
green door. Touch a spike or fall out the bottom and you respawn at
once — your deaths counter is the only cost. A fresh platforming genre
alongside the endless-runner `pixel-platformer-infinite`.

## Features

- 6-room campaign **First Steps → Gauntlet** — flat ground gives way
  to shallow pits, then deadly spike-floored gaps.
- Tight, readable platforming physics: variable-height jumps (hold the
  jump button for more lift), **coyote time** (a grace window to jump
  just after leaving a ledge) and a **jump buffer** (a press just
  before landing still fires).
- A **mid-air dash** — one horizontal burst per jump, refreshed the
  moment you touch down — for crossing wider gaps and grabbing the
  high gems in style.
- A 240 Hz collision substep inside the variable-dt loop, so a fast
  fall never tunnels through a thin ledge.
- Optional gems tucked above the route reward a confident dash-jump.
- Four-button pad — hold **◀ ▶** to move, tap **▲** to jump, **»** to
  dash — plus arrow keys / WASD, Space and Shift on desktop.
- Per-room best score and progressive unlocks saved to `localStorage`.
- Chunky tile art — bevelled blocks, glinting spikes, a glowing exit
  door.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, a touch
  control pad below the play field.
- Verified: 43 checks — every room is a 15×15 grid with one spawn and
  one exit, the hero spawns clear of walls and spikes and idles safely
  on the spawn ledge; a jump lifts the hero and gravity returns it; a
  mid-air dash fires once and bursts horizontally; holding into a wall
  never tunnels through it; a spike costs a death and respawns; the
  exit wins; **a walk-and-jump bot clears all 6 rooms**; plus a UI
  smoke pass with pad input.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4397
```

Then visit `http://127.0.0.1:4397/index.html`.

## Play

- Hold **◀ ▶** (or arrow keys / A·D) to run.
- Tap **▲** (or Up / Space) to jump — hold it for a higher leap.
- Tap **»** (or Shift / X) to air-dash once; landing refreshes it.
- Reach the green door. Spikes and the pit below respawn you — the
  room itself never ends, so keep trying.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — tile rooms, hero physics (run / variable jump / dash /
  coyote / buffer), AABB tile collision, spikes, gems, win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, bevelled tiles, spikes, exit door, the hero.
- `js/game.js` — screen flow, four-button pad + keyboard input, RAF
  loop, save.
