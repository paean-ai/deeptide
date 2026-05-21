# Pixel DigDug

A pixel-art tribute to **Dig Dug** — carve tunnels through the dirt
with your boots, then pump the enemies you line up with until they
pop. Falling rocks score even bigger if they squash a chaser. A fresh
dig-and-pump arcade alongside the other `samples/` pixel games.

## Features

- 9-level campaign **Topsoil → Inferno** scaling enemy count (2 → 7),
  enemy speed (1.8 → 4.2 cells / sec) and rock count (0 → 6) — the
  Magma, Core and Inferno depths cap the descent.
- 16 × 20 cell pit (cell 20 px) with two-row sky band on top; every
  dirt cell becomes a tunnel once you walk through it (+1 score per
  cell dug).
- Each enemy walks tunnels toward you on a greedy gradient with a
  "no-reverse-unless-dead-end" rule; while inflated they're frozen.
- **Pump mechanic**: hold the ★ button while facing an enemy in your
  line of sight (a single-cell-wide tunnel between you, no dirt in
  the way) — three pumps pops it for **+200**. Release before the
  third pump and the enemy slowly deflates.
- **Rocks** fall when the cell directly below them becomes empty;
  any enemy or you they cross is squashed (**+400 per enemy**).
- 3 lives + invuln blink on respawn; HUD shows level, hearts, score.
- On-screen five-button strip (←↓↑→★) with multi-touch pointer
  tracking PLUS arrow keys / WASD / Space for pump on desktop.
- Level select with progressive unlocks; per-level best score saved
  to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 38 checks — all 9 levels build placing the full enemy
  count, enemy speed only ever rises across the campaign, and every
  level config is sane; plus a load check that all four scripts run
  cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4331
```

Then visit `http://127.0.0.1:4331/index.html`.

## Play

- Hold ← / ↓ / ↑ / → (or the on-screen pad) to dig in that direction.
- Stop alongside an enemy with no dirt between you and **hold ★**
  to inflate them — three pumps pops them for +200.
- Lure an enemy under a rock, then dig the dirt beneath the rock
  to drop it for a +400 squash bonus.
- Three lives. Contact with an enemy or a falling rock costs one.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — grid + tile model, dig + walk, enemy AI, pump rules,
  rock physics, lives / win logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, dirt + tunnel + rock tiles, player +
  enemies with inflation gauge, dashed pump hose, on-screen strip.
- `js/game.js` — screen flow, multi-touch + keyboard input, RAF loop,
  save.
