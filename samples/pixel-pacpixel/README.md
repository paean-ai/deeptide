# Pixel Pac-Pixel

A pixel-art tribute to **Pac-Man** — eat every pellet in a small hand-
designed maze, avoid the ghosts, and grab a power-pellet at any corner
to briefly turn the tables. A fresh maze-chase arcade alongside the
other `samples/` pixel games.

## Features

- 6-level campaign **Wakka → Finale** scaling Pac speed
  (×4.4 → ×5.4 cells / sec) and ghost speed (×3.6 → ×5.4), with a
  shrinking post-power panic window (8.5 s → 3.5 s).
- One hand-designed 17 × 20 maze: corridors thick with pellets, four
  corner power-pellets, two side tunnels that wrap left ↔ right.
- **Two ghosts**: Blinky (red, chases Pac's current cell) and Pinky
  (pink, aims four cells ahead of Pac). Both run from the central
  home cells and respect "no reverse" when picking turns; in panic
  mode they move slower and pick random forward turns; once eaten,
  they become **eyes** moving 1.6× speed back to home before
  re-spawning as chasers.
- Power-pellet pulse + panic-mode end-blink so timing reads at a
  glance. Eating a panicked ghost = +200.
- Pac mouth animates when moving; tunnel wrap on the two side caps;
  240 Hz substep inside the variable-dt RAF tick so a fast turn at
  a corner lands cleanly on the cell-centre direction-pick.
- 3 lives + invuln blink on respawn; HUD shows lives as hearts,
  pellet count and live score. Per-level best score persisted to
  `localStorage`.
- Drag (16 px threshold) or swipe to queue direction; the next
  direction kicks in at the next cell-centre that allows it.
  Arrow keys / WASD on desktop.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 73 mechanics checks — maze sanity (every row width
  matches), every level builds with 4 power-pellets and 2 ghosts and
  in-range Pac start, the maze pellet count exactly matches the
  initial counter, setDir routes Pac into open corridors and a wall
  blocks movement, eating a pellet increases score by 10, eating a
  power-pellet starts the panic timer + flips ghosts to panic, die
  decrements lives and three deaths trigger game-over, eating the
  last pellet wins the level, finalScore adds 100 per remaining life.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4323
```

Then visit `http://127.0.0.1:4323/index.html`.

## Play

- Drag or use arrow keys / WASD to choose a direction; the turn
  queues until the next corridor allows it.
- Eat every pellet. Grab a corner power-pellet to flip ghosts to
  blue panic — catch one for **+200**.
- Tunnel through the two side caps to wrap to the opposite cap.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — maze layout, levels, Pac + ghost movement, pellet /
  power-pellet, panic timer, win + life loss flow.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, brick walls, pellets / power-pellet pulse,
  Pac with animated mouth, ghosts (chase / panic / eyes), HUD.
- `js/game.js` — screen flow, swipe / keyboard input, RAF loop with
  240 Hz substep, save.
