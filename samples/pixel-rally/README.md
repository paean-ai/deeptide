# Pixel Rally

A pixel-art **Pong**. Nine bouts against a CPU opponent that gets faster and
sharper each match. A fresh paddle-and-ball arcade genre alongside the other
`samples/` pixel games.

## Features

- 9 opponents on a difficulty curve from Rookie to Legend — each tougher CPU
  is faster and predicts the ball's bounce path further ahead; the Ace,
  Phenom and Legend rivals barely miss.
- First to 5 wins the match. The ball speeds up slightly with every paddle
  hit and a hit off-centre angles the return — go for the edge to defeat the
  predictive CPUs.
- Drag anywhere on the court to move your paddle; works cleanly on mobile and
  desktop.
- Per-bout completion marks and progressive unlocks, English/中文 toggle,
  saved to `localStorage`.
- Verified: 38 checks — all 9 bouts build, CPU speed and ball speed only
  ever rise across the campaign, and every opponent's predict factor stays
  in range; plus a load check that all four scripts run cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4258
```

Then visit `http://127.0.0.1:4258/index.html`.

## Play

- Drag your finger or mouse across the court to slide your bottom paddle.
- Hit the ball with the edge of your paddle to angle the return — the centre
  sends it straight back.
- First side to score 5 points wins the bout.

## Structure

- `index.html` - shell, title / opponent-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - paddle / ball physics, CPU AI, scoring.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - court, paddles, ball, midline, score watermark.
- `js/game.js` - real-time loop, drag input, save.
