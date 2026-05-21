# Pixel Aegis

A pixel-art **rotate-the-shield core defence**. A core sits at the centre,
ringed by shooters firing straight at it. Drag a shield arc around the core —
a shot you block rebounds and strikes the shooter that fired it. A fresh
real-time defence genre alongside the other `samples/` pixel games.

## Features

- A 6-siege campaign — Picket, Cordon, Volley, Crossfire, Barrage, Onslaught
  — with shooters, fire rate and shooter mix climbing across the run.
- Blocking is offence: every shot you catch with the shield rebounds onto its
  shooter, so a clean defence is also how you clear the ring.
- Three shooter types — the Gunner (two blocks to fell), the Twin (three),
  the Burst (looses a three-shot salvo) — each demanding a different read.
- A PULSE: tap to flash the shield full-circle on a cooldown, catching a
  whole volley at once when the ring is overrun.
- Three-star scoring by core health left, per-siege stars and progressive
  unlocks; drag on phone, mouse on desktop. English/中文 toggle, saved to
  `localStorage`.
- Verified: 37 checks — angle maths, the shield block / rebound, shooter
  health (gunner / twin / burst), uncovered shots reaching the core, the
  pulse, win and loss; a competent bot clears all 6 sieges; plus a 4-script
  smoke test that drives a siege to a win and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4453
```

Then visit `http://127.0.0.1:4453/index.html`.

## Play

- Drag anywhere to swing the shield arc around the core.
- A shot that crosses the shield is blocked — and the blow rebounds to strike
  the shooter that fired it. A shot you miss damages the core.
- Tap **PULSE** to flash the shield full-circle for a moment — it catches
  every shot at once, then needs to recharge.
- Fell every shooter before the core falls. Keep the core healthy for stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the sieges, the shield, shots, blocking and the pulse.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - the circular arena, shield, shooters, core, title art.
- `js/game.js` - screen flow, real-time loop, drag-to-aim input, save.
