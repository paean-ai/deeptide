# Pixel Cycle

A pixel-art **light-cycle duel** in the spirit of Tron. Two cycles leave trails
of light wherever they ride; the first one to crash into ANY trail or wall is
out. A fresh territory-survival genre alongside the other `samples/` pixel
games.

## Features

- 6 opponents on a difficulty curve from Rookie to General — CPU AI ranges
  from death-only avoidance to a flood-fill space-maximiser.
- Best of three rounds wins the match; a max round cap keeps stalemates from
  going on forever.
- On-screen LEFT / RIGHT turn buttons work cleanly on mobile and desktop.
- Per-duel completion marks and progressive unlocks, English/中文 toggle,
  saved to `localStorage`.
- Verified: 13 mechanics checks (cycle movement, wall + trail collision,
  round end, match end) + a winnability check (a flood-fill bot beats 4/6
  opponents — every match terminates) + a UI input check.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4260
```

Then visit `http://127.0.0.1:4260/index.html`.

## Play

- Tap LEFT or RIGHT to turn your cycle relative to its current direction.
- Trails are walls — driving into any trail or the field edge ends the round.
- Win 2 rounds out of up to 7 to clear the duel.

## Structure

- `index.html` - shell, title / opponent-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - grid, cycles, trails, CPU AI, round flow.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - field, trails, cycle heads, on-screen turn buttons.
- `js/game.js` - real-time loop, input, save.
