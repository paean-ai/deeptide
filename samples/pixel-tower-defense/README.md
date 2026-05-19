# Pixel Tower Defense

A dependency-free HTML Canvas tower defense sample in a unified pixel-art style.

## Features

- 4 tower families, each with a tier-2 upgrade and a branching tier-3 choice
  (8 distinct endgame towers).
- 7 enemy types with armor, magic resist, flying, healing, and bosses —
  physical vs. magic damage actually matters.
- Status effects: slow, freeze, splash, armor shred, chain lightning.
- 7 hand-built maps (grass / snow / lava themes) with 14–26 waves of rising
  difficulty, plus an endless mode on every map.
- Per-tower targeting modes (first / last / strongest / closest), sell & refund.
- Game speed 1x/2x/3x, pause, early-wave-call gold bonus.
- Star rating, level unlocks, `localStorage` progress save.
- Responsive desktop + mobile layout, touch placement, English/中文 toggle.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4181
```

Then visit `http://127.0.0.1:4181/index.html`.

## Play

- Tap a tower in the build bar, then tap a grass tile to build it.
- Tap a placed tower to upgrade, retarget, or sell it.
- Press **Start Wave** to begin; calling waves early grants bonus gold.
- Stop every enemy before they drain your core's lives.

## Structure

- `index.html` - screen shells (title / level select / game).
- `css/style.css` - responsive pixel UI, HUD, build bar, overlays.
- `js/i18n.js` - English/Chinese strings and display names.
- `js/data.js` - tower, enemy, level, and wave definitions.
- `js/art.js` - pixel sprite, terrain, and tower rendering.
- `js/game.js` - engine, combat, input, UI, and screen flow.
