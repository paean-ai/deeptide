# Pixel Dots & Boxes

A pixel-art take on the pencil-and-paper classic **Dots and Boxes**, played
against an AI. Draw edges between the dots — close the fourth side of a box to
claim it and take another turn. A fresh paper-game genre alongside the other
`samples/` pixel games.

## Features

- Full Dots and Boxes rules on a 5×5 grid of boxes — completing a box claims it
  and grants an extra turn, so a good chain can run several boxes in a row.
- A real opponent: a heuristic AI that always takes free boxes, plays a safe
  edge when one exists, and — on Hard — sacrifices the *shortest* chain when
  forced to give boxes away.
- Three difficulties: Easy takes boxes only, Medium also avoids gifts, Hard
  adds shortest-chain sacrifice play.
- The last edge drawn is highlighted; the live box count is always shown.
- Per-difficulty win / loss / draw records, saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4243
```

Then visit `http://127.0.0.1:4243/index.html`.

## Play

- Pick a difficulty, then tap between two neighbouring dots to draw that edge.
- Drawing the fourth edge of a box claims it for you — and you move again.
- Avoid drawing the third edge of a box: that hands it to your opponent.
- When every box is claimed, the player with the most boxes wins.

## Structure

- `index.html` - shell, title (difficulty pick) / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - board rules, box claiming and the heuristic AI.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, dots, edges, claimed boxes.
- `js/game.js` - turn flow, AI hand-off, stats save.
