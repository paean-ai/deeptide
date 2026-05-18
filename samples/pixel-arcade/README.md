# Pixel Arcade

A dependency-free HTML Canvas mini-game pack — five quick skill games sharing
one pixel cabinet.

## The games

- **Sky Flap** — tap to flap through scrolling pipe gaps.
- **Fruit Catch** — drag a basket to catch fruit and dodge bombs (3 lives).
- **Reflex Tap** — hit shrinking targets before they expire (3 lives).
- **Tower Stack** — drop sliding blocks; overhang gets trimmed away.
- **Pixel Dash** — an endless runner; tap to jump (and double-jump) obstacles.

## Features

- One shared shell: hub, game loop, input, particles, screen shake, pause.
- Per-game high scores and bronze/silver/gold medals with a gold tally.
- Consistent pixel-art direction across all five games.
- `localStorage` best-score save, English/中文 toggle.
- Responsive desktop + mobile: a fixed 480x720 play field letterboxed to fit,
  pointer and keyboard (Space) controls.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4186
```

Then visit `http://127.0.0.1:4186/index.html`.

## Play

- Pick a game from the arcade hub.
- Tap once to start, then play — each game ends on a miss or when lives run out.
- Beat your best score and chase a gold medal in all five.

## Structure

- `index.html` - hub and play screens, overlays.
- `css/style.css` - responsive pixel UI and game cards.
- `js/i18n.js` - English/Chinese strings.
- `js/art.js` - shared pixel-art helpers and per-game sprites.
- `js/game.js` - shell (hub, loop, scoring, medals) and all five game modules.
