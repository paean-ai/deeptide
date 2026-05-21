# Pixel Duelist

A pixel-art **parry/dodge timing duel**. A foe winds up an attack along a
timing track — a blue slash must be **parried**, an amber thrust must be
**dodged**, and only an input landed in the react window near the strike
counts. Catch it in the last sliver for a **PERFECT**. Fill the foe's posture
bar to stagger them, then **EXECUTE** for a point of health. A fresh
reaction-duel boss-rush genre alongside the other `samples/` pixel games.

## Features

- A 6-foe boss rush — Recruit, Sentinel, Duelist, Warden, Champion, Revenant —
  with windups tightening, react windows shrinking and the share of dodge
  attacks climbing from one in five up to over half.
- Read the blade, don't mash: a misread or a too-early press is spent and the
  strike still lands. A PERFECT (the last 190 ms) is worth double posture.
- Posture builds from clean guards; fill it to stagger the foe, then a single
  EXECUTE strips a health point. Miss the execute window and they recover.
- One thumb on each side — tap the left half to parry, the right to dodge;
  the staggered foe turns the whole base into an EXECUTE button. Arrow keys
  or Z / X plus Space work on desktop.
- Three-star scoring (flawless / clean / cleared), per-foe stars and
  progressive unlocks, English/中文 toggle, saved to `localStorage`.
- Verified: 61 checks — a perfect-timing bot clears all 6 foes flawlessly, a
  passive player is defeated by every foe, a parry-only run gets hit on the
  thrust-heavy foe, too-early presses / misreads / stagger / execute / a
  missed execute window all behave; plus a 4-script load-and-render smoke
  test driving title → select → duel → result.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4421
```

Then visit `http://127.0.0.1:4421/index.html`.

## Play

- Watch the marker race down the timing track toward the strike line.
- Blue slash: tap the left half (PARRY). Amber thrust: tap the right half
  (DODGE). Land it inside the lit react band — the last sliver is a PERFECT.
- Each clean guard adds posture. Fill the posture bar and the foe staggers —
  tap anywhere to EXECUTE before the window drains.
- Drop the foe's health to zero to win. Take no hits for three stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - foe roster, attack generation, parry/dodge/execute rules.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, blocky knights, the timing track, HUD, title art.
- `js/game.js` - screen flow, real-time loop, tap / keyboard input, save.
