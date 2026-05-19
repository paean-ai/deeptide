# Pixel Codebreaker

A pixel-art Mastermind. Crack the hidden colour code by deduction, reading the
feedback pegs after every guess. A fresh code-breaking puzzle genre alongside
the other `samples/` pixel games.

## Features

- 9-level campaign on an escalating curve — longer codes (3 → 5 pegs), more
  colours (5 → 7) and fewer attempts deeper in.
- Classic Mastermind feedback: a solid peg for each right colour in the right
  spot, a hollow peg for a right colour in the wrong spot — repeated colours
  scored correctly.
- Every code is generated from a seed, so each level is the same puzzle every
  time.
- Tap colours to build a guess, tap a placed peg to clear it, then submit.
- 3-star rating by how few guesses you need; the code is revealed if you run
  out of tries.
- Level select with progressive unlocks, per-level star records, all saved to
  `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4240
```

Then visit `http://127.0.0.1:4240/index.html`.

## Play

- Tap a colour from the palette to drop it into your guess; tap a placed peg
  to remove it.
- Submit a full guess to score it — solid feedback pegs are exact matches,
  hollow ones are right-colour-wrong-place.
- Deduce the code before your tries run out. Fewer guesses earn more stars.

## Structure

- `index.html` - shell, title / level-select / game screens, win & lose overlays.
- `css/style.css` - responsive 360:480 shell, HUD, level grid.
- `js/data.js` - levels, seeded code generation, feedback scoring.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, code pegs, feedback clusters.
- `js/game.js` - guess building, win/lose detection, scoring, save.
