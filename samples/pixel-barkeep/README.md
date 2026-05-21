# Pixel Barkeep

A pixel-art **Tapper**-style serving arcade. Patrons stride down four
counters toward your bar — slide a mug down a counter to shove the
nearest one back, and shove them clean off the end to serve them. Let
a patron reach the bar and they grab you. A fresh timing-arcade genre
alongside the other `samples/` pixel games.

## Features

- Endless escalating rounds — each round sends more patrons down the
  four counters, faster.
- Three patron kinds for real reaction variety: a steady **regular**,
  a crawling **sluggard**, and a fast **rowdy** worth the most points.
- Each mug shoves the nearest patron back a fixed distance; a patron
  near the bar takes several quick mugs to drive off, a fresh one just
  one — so you triage the four counters under pressure.
- Catch the **empty mugs** that come sliding back: a mug returning to
  a counter you're standing at is worth a tidy bonus.
- One-tap control: tap a counter to step there and slide a mug in a
  single motion (or ◀▲▼ keys — Up/Down to move, Space to pour).
- 3 lives; a patron that reaches the bar costs one. A round bonus and
  a per-run best score, saved to `localStorage`.
- Chunky pixel art — a wood-grain bar, foam-topped mugs, an aproned
  barkeep, distinct patrons.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 22 checks — a fresh game starts with three lives on round
  one; the barkeep lane clamps; a pour slides one mug and is blocked
  on cooldown; a mug shoves the nearest patron back and a patron
  shoved off the end is served, scored and counted, leaving a
  returning empty mug; an empty mug caught in the barkeep's lane
  scores exactly +15 while one in another lane scores nothing; a mug
  poured into an empty lane is discarded; a patron reaching the bar
  costs a life and three end the run; a cleared round advances; the
  same seed plays identically; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4389
```

Then visit `http://127.0.0.1:4389/index.html`.

## Play

- Tap a counter to step to it and slide a mug down it (or use Up /
  Down to move and Space to pour).
- A mug shoves the nearest patron on that counter back — shove a
  patron off the far end to serve them.
- Catch the empty mugs sliding back for a bonus.
- Never let a patron reach the bar — three do and it's last call.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — counters, patrons + kinds, mug sliding / shove / serve
  resolution, round escalation, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, tavern + wood counters, patrons, foam mugs,
  the aproned barkeep, HUD.
- `js/game.js` — screen flow, tap / key input, RAF loop, save.
