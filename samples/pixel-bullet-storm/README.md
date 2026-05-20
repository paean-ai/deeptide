# Pixel Bullet Storm

A pixel-art bullet-hell shooter. Thread your ship through dense, overlapping
bullet patterns — your sprite is big, but only the bright dot at its centre can
be hit. A fresh danmaku genre alongside the other `samples/` pixel games.

## Features

- True bullet-hell hitbox: the only thing that kills you is a tiny core dot,
  far smaller than the ship sprite around it.
- Mobile-first control — drag anywhere and the ship rides above your finger so
  it's never hidden; arrow keys / WASD also work on desktop.
- Five enemy archetypes with distinct emitters — aimed bursts, even rings,
  spreading fans, rotating spirals, and a stationary lattice that drops a
  full horizontal wall of seven bullets at a time.
- A multi-phase boss every 5th wave: its patterns escalate as its health drops,
  from spinning rings to a four-armed spiral barrage.
- Grazing — skimming a bullet without being hit scores bonus points.
- Screen-clearing bombs (limited stock) that wipe every bullet and stagger
  every enemy.
- A kill-fed power meter that widens your shot from one stream up to a
  four-way spread.
- Endless escalating waves, 3 lives, parallax starfield, `localStorage` best
  score.
- English / 中文 toggle.
- Responsive 360:480 playfield — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4223
```

Then visit `http://127.0.0.1:4223/index.html`.

## Play

- Drag to fly. The white-and-pink dot in the ship's centre is your hitbox —
  keep *that* clear, not the whole sprite.
- Your cannon fires automatically; destroy enemies to climb the power meter.
- Skim close to bullets to graze them for bonus score.
- Tap BOMB (or press Space) to clear the screen when a pattern boxes you in.
- Survive the storm and push for the deepest wave.

## Structure

- `index.html` - shell, title / game / game-over screens.
- `css/style.css` - responsive 360:480 shell, HUD, control bar.
- `js/data.js` - enemy archetypes, boss and wave generation.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - starfield, ship bitmap, enemy / boss / bullet rendering.
- `js/game.js` - bullet patterns, dodging, bombs, waves, scoring, save.
