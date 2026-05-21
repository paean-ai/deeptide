# Pixel Puck

A pixel-art **air-hockey** match against a CPU opponent. Drag your mallet
freely in 2D — slam the puck through the opening at the top to score. First
to 5 wins. A fresh 2D paddle-and-puck genre alongside the other `samples/`
pixel games (distinct from the 1D paddle of `pixel-rally`).

## Features

- 9-match campaign from Rookie to Apex — each tougher CPU is faster and
  predicts the puck's bounce path further ahead. The Phantom, Oracle and
  Apex opponents read the puck almost perfectly.
- 2D mallet motion: drag anywhere in the lower half to slide your paddle.
- Continuous puck physics with side-wall bounces, an air-cushion friction,
  and goal openings at top and bottom.
- Per-match completion marks and progressive unlocks, English/中文,
  `localStorage` save.
- Verified: 29 checks — all 9 matches build, CPU speed only ever rises
  across the campaign, and every opponent's predict / aim factors stay in
  range; plus a load check that all four scripts run cleanly.

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
