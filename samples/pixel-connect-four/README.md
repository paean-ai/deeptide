# Pixel Connect Four

A pixel-art Connect Four against a minimax AI. Drop discs into the grid and
line up four before the machine does. A fresh board-game-versus-AI genre
alongside the other `samples/` pixel games.

## Features

- Classic 7×6 Connect Four — line up four discs across, down or diagonally.
- A real opponent: an alpha-beta minimax AI with a positional heuristic.
- Four difficulties set the AI's search depth — Easy (2), Medium (4),
  Hard (6) and Expert (8 plies), all with centre-first move ordering for
  fast alpha-beta pruning.
- A satisfying gravity drop animation for every disc, and the winning line is
  highlighted at the end.
- Per-difficulty win / loss / draw records, saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4239
```

Then visit `http://127.0.0.1:4239/index.html`.

## Play

- Pick a difficulty, then tap a column to drop your red disc.
- The disc falls to the lowest free slot; the AI answers with a yellow disc.
- First to connect four in any direction wins; a full board is a draw.
- Beat the Expert AI for real bragging rights.

## Structure

- `index.html` - shell, title (difficulty pick) / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - board rules, win detection and the minimax AI.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, board panel, discs.
- `js/game.js` - turn flow, drop animation, AI hand-off, stats save.
