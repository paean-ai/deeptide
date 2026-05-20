# Pixel Helix

A pixel-art take on the **Helix Jump** rotate-the-tower-drop-the-ball
mechanic, rendered as a clean top-down view of a single disc at a time
with a faint preview of the disc below. Drag to spin the disc; the
ball auto-bounces and drops through any gap that aligns under it. A
fresh hyper-casual genre alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Spring → Singularity** scaling tower depth
  (**10 → 30** discs), bounce cadence (**1.6 → 3.3 Hz**), and danger
  density (0 → 3 spikes per disc).
- Each disc is **twelve angular segments**: solid (purple, bounces),
  gap (dark, drops you through), and danger (red spikes, fatal).
  Layouts are seeded per level so the run is reproducible.
- Real-time bounce cycle: when the ball's down-peak lands over a gap
  it falls; over a spike it dies; over a solid it bounces and the
  combo resets after the second consecutive bounce on the same disc.
- **Combo scoring**: every gap clears 25 + 10 × (combo - 1), so a
  three-disc chain banks 25 + 35 + 45 = 105 points.
- Drag horizontally (80 px == one segment) or use arrow keys / WASD
  to spin the disc; the next disc shows faintly below so you can plan
  two steps ahead.
- Level select with progressive unlocks; per-level best score saved
  to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: **499 mechanics checks** — every level builds with each
  disc holding exactly twelve segments and the configured gap /
  danger budgets; segment-under-ball calculus invariant under disc
  rotation; solid bounce keeps the ball on the same disc; gap drops
  cleanly to the next disc with combo scoring exactly equal to
  25 + 35 + 45 + ...; danger ends as a loss; descending past the
  final disc wins with a 500-point bonus; rotate is a no-op while
  a fall transition is in flight.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4327
```

Then visit `http://127.0.0.1:4327/index.html`.

## Play

- Drag left or right (or arrow keys / WASD) to spin the disc.
- A pale **GAP** under the ball at the bounce-down peak drops you to
  the next disc — chain gaps for combo points.
- A red **SPIKE** under the ball at the down-peak is fatal.
- Descend every disc to clear the tower.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — disc / tower model, segment-under-ball math, bounce
  cycle + fall transition, combo + win / lose logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, top-down disc with three segment types,
  spike teeth on danger segments, bouncing pixel ball, HUD with
  combo chip.
- `js/game.js` — screen flow, drag + keyboard input, RAF loop, save.
