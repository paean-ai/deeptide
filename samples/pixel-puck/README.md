# Pixel Puck

A pixel-art **air-hockey** match against a CPU opponent. Drag your mallet
freely in 2D — slam the puck through the opening at the top to score. First
to 5 wins. A fresh 2D paddle-and-puck genre alongside the other `samples/`
pixel games (distinct from the 1D paddle of `pixel-rally`).

## Features

- 6-match campaign from Rookie to Master — each tougher CPU is faster and
  predicts the puck's bounce path further ahead.
- 2D mallet motion: drag anywhere in the lower half to slide your paddle.
- Continuous puck physics with side-wall bounces, an air-cushion friction,
  and goal openings at top and bottom.
- Per-match completion marks and progressive unlocks, English/中文,
  `localStorage` save.
- Verified: 15 mechanics checks (serve launch, wall bounces, paddle
  collision in the paddle's frame, top/bottom goal scoring, match end at
  points-to-win, half-of-field clamps) + 12 live-system checks (CPU paddle
  tracks the puck, puck moves under play, all six levels) + 2 UI input
  checks.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4266
```

Then visit `http://127.0.0.1:4266/index.html`.

## Play

- Drag a finger anywhere in the lower half to move your mallet — it follows
  the touch point in 2D.
- The puck bounces off the side walls and off both mallets. Hit it through
  the top goal opening to score; if it enters your goal, the CPU scores.
- First to 5 wins the match.

## Structure

- `index.html` - shell, title / opponent-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - puck physics, paddle collision, CPU AI, scoring.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - table, mallets, puck, goal openings.
- `js/game.js` - real-time loop, drag input, save.
