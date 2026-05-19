# Pixel Reversi

A pixel-art Reversi (Othello) against a minimax AI. Trap the opponent's discs
between yours to flip them, and finish owning the most of the board. A board
game versus AI alongside the other `samples/` pixel games.

## Features

- Full Othello rules on an 8×8 board — flank a line of opponent discs to flip
  the whole line; pass automatically when you have no legal move.
- A real opponent: an alpha-beta minimax AI with the classic positional
  weighting (corners prized, the squares beside them avoided) plus a mobility
  term.
- Three difficulties set the AI's search depth — Easy (1), Medium (3) and
  Hard (4 plies).
- Legal moves are dotted on your turn; the last move and the live disc count
  are always shown.
- Per-difficulty win / loss / draw records, saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4241
```

Then visit `http://127.0.0.1:4241/index.html`.

## Play

- Pick a difficulty, then tap a dotted square to place your dark disc.
- Every opponent disc trapped in a straight line between your new disc and
  another of yours flips to your colour.
- If you have no legal move, your turn passes; when neither side can move the
  game ends.
- Hold the corners — own the most discs to win.

## Structure

- `index.html` - shell, title (difficulty pick) / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - board rules, flip logic and the minimax AI.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, board felt, discs.
- `js/game.js` - turn flow, passes, AI hand-off, stats save.
