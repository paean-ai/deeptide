# Pixel Idle Quest

A pixel-art **idle monster-slaying RPG**. A lone hero grinds an
endless dungeon — tap the monster to strike it, and your squires deal
damage on their own even while you rest. Gold buys a sharper blade and
more squires; stuck on the curve, ascend to trade your run for relics
that permanently multiply all your damage. A fresh idle genre
alongside the other `samples/` pixel games.

## Features

- An endless dungeon: clear 10 monsters to descend a stage; every 5th
  stage is an **elite** floor with a tougher, gold-crowned foe and a
  fatter purse.
- Tap **and** idle: tapping the monster lands your blade for big
  hits, while squires grind it down automatically — so the run
  progresses whether you're playing actively or away.
- Two upgrade tracks with geometric costs — **Blade** (tap damage)
  and **Squire** (auto damage) — tuned so progress is steady early
  and gently slows, exactly the idle curve. (A play-out simulation
  confirms a 12-minute run lands around stage 60–70 with no runaway
  and no number overflow.)
- **Ascend** once you've reached stage 10+: reset the run for relics
  that give a permanent ×damage multiplier, then re-climb far faster.
- **Offline catch-up**: the gold your squires would have ground out
  while the tab was closed is granted on return (capped at 8 hours).
- The whole run auto-saves to `localStorage`.
- Big readable numbers with K / M / B / T… suffixes.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 29 checks — fresh and restored game state; HP and gold
  curves grow with an elite spike and stay finite deep in; tapping
  and auto-DPS both damage and kill monsters; ten kills advance the
  stage; upgrades cost-gate, deduct gold and raise damage; ascend
  unlocks past stage 6, grants relics and resets the run with a
  permanent boost; saves round-trip and reject corrupt fields;
  offline gold is granted and capped; a 12-minute economy simulation
  lands at a sane stage with finite numbers; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4413
```

Then visit `http://127.0.0.1:4413/index.html`.

## Play

- Tap the **monster** to strike it with your blade.
- Buy **Blade** to hit harder on tap, **Squire** to raise the damage
  that lands on its own.
- Clear 10 monsters to descend a stage; the deeper you go, the richer
  the gold.
- When the climb slows, **Ascend** — bank relics and start over far
  stronger.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — economy curves, combat, upgrades, ascension, offline
  earnings, number formatting.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, dungeon, the hero, monsters by stage, HP bar,
  HUD.
- `js/game.js` — screen flow, RAF loop, tap input, auto-save with
  offline catch-up.
