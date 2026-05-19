# Pixel Solitaire

A pixel-art Klondike solitaire — the classic patience card game, built for
one-tap play. A fresh card-game genre alongside the other `samples/` pixel
games.

## Features

- Full Klondike rules: seven tableau columns, four foundations, a stock and
  waste, build-down-by-alternating-colour, Kings to empty columns.
- One-tap play tuned for mobile — tap any card and it travels to its best legal
  spot (a foundation if it fits, otherwise a tableau column); tap a buried
  card to move it together with the run beneath it.
- Tap the deck to draw; tap again when it's empty to recycle the waste — passes
  are unlimited.
- Unlimited **undo** — every move and draw is reversible.
- Face-down cards flip automatically when they reach the top of a column.
- Win detection with a confetti burst; a move counter and timer.
- `localStorage` records games won, best time and best move count.
- English / 中文 toggle.
- Responsive 360:480 table — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4227
```

Then visit `http://127.0.0.1:4227/index.html`.

## Play

- Tap the deck (top-left) to turn a card onto the waste pile.
- Tap a card to send it to a foundation or onto another tableau column — the
  game picks the best legal destination.
- Tap a face-up card partway down a column to move it and everything below it.
- Build each foundation up from Ace to King in one suit. Clear all 52 to win.
- Stuck? Use UNDO, or deal a NEW game.

## Structure

- `index.html` - shell, title / game screens, win overlay.
- `css/style.css` - responsive 360:480 shell, HUD, control bar.
- `js/data.js` - suit and card definitions, board geometry.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - felt backdrop, suit pips, card and slot rendering.
- `js/game.js` - Klondike rules, one-tap moves, undo, win detection, save.
