# Pixel Card Spire

A dependency-free HTML Canvas roguelike deck-builder with a data-driven card engine.

## Features

- 48-card pool (starter / common / uncommon / rare) plus a card upgrade system,
  all resolved by a small data-driven effect engine (damage, block, draw,
  energy, strength, vulnerable, weak, poison, heal, persistent powers).
- 11 enemies with intent telegraphs, scaling patterns, blocking, buffs,
  debuffs, thorns, plus 2 elites and a multi-pattern boss.
- 12-floor branching map: monsters, elites, shops, campfires, events, boss.
- Relics (14) with combat-start, on-kill, passive, and bloodied triggers.
- Rewards, card removal/upgrade, shops, and random events between fights.
- Block / strength / vulnerable / weak / poison status system.
- `localStorage` run save with continue, English/中文 toggle.
- Responsive desktop + mobile: scalable battle scene, scrollable card hand,
  tap-to-arm / tap-target controls.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4182
```

Then visit `http://127.0.0.1:4182/index.html`.

## Play

- Each turn you draw 5 cards and have 3 Energy. Play cards to attack and defend.
- Attack cards ask for a target — tap the card, then tap an enemy.
- Read each enemy's intent above its head before ending your turn.
- Defeat **The Warden** on floor 12.

## Structure

- `index.html` - screen shells (title / map / combat / reward / rest / shop / event).
- `css/style.css` - responsive pixel UI, card styling, map, overlays.
- `js/i18n.js` - English/Chinese strings and auto-generated card text.
- `js/content.js` - card, enemy, relic, and event definitions.
- `js/art.js` - pixel creature and battle-scene rendering.
- `js/game.js` - run state, map, combat engine, screen flow.
