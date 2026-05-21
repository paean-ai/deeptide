# Pixel Boxing

A pixel-art **Punch-Out**-style timing duel. Read the opponent's
wind-up tell, dodge the correct way to make the punch whiff, then
counter-punch through the stagger window for triple damage. A fresh
rhythm-combat genre alongside the beat-'em-up `pixel-street-brawl`.

## Features

- 9-bout campaign **Glass Joe → Grand Champ** scaling foe HP
  (60 → 280), hit power (12 → 40) and — critically — wind-up time
  (1.10 s → 0.36 s), so later foes give far less reaction time. The
  closing trio — Steel Vega, Crusher Cain, Grand Champ — barely
  telegraph at all.
- A clean opponent state machine: **idle → windup** (picks a side,
  a glowing arm shows the tell) **→ strike** (the punch lands now)
  **→ stagger** (if you dodged correctly — the counter window) or
  **recover** (if it hit) **→ idle**.
- Dodge the **opposite** way to the tell to evade and stagger the
  foe. A **counter punch** during the stagger window deals 26 +
  combo damage and builds a combo; a **jab** on an idle foe chips 6;
  punching into a wind-up is wasted and resets your combo.
- **Block** cuts an unavoidable hit to 35 % damage.
- Four-button control pad (DODGE ◀ / DODGE ▶ / BLOCK / PUNCH) plus
  arrow keys / WASD / Space on desktop.
- Two HP bars, floating hit / dodge / counter callouts, KO ends the
  bout. Per-bout best score saved to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 47 checks — all 9 bouts build with the declared foe HP,
  HP only rises and wind-up time only shrinks across the campaign,
  every bout starts in a fresh state; plus a load check that all
  four scripts run cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4337
```

Then visit `http://127.0.0.1:4337/index.html`.

## Play

- Watch the foe's arms. When one winds up and glows, that's the
  punch side.
- Tap **DODGE** the *opposite* way to slip the punch — the foe
  staggers.
- Tap **PUNCH** during the stagger to land a triple-damage counter.
- Can't read it in time? **BLOCK** to soften the blow.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — opponent state machine, dodge / counter / jab /
  block resolution, 9 bout definitions, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, ring backdrop, foe + player boxers with
  glove poses + wind-up tell glow, HP bars, callouts, control pad.
- `js/game.js` — screen flow, button + keyboard input, RAF loop, save.
