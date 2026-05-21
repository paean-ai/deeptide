# Pixel Elemancer

A pixel-art **turn-based elemental-form battle RPG**. One hero channels four
elemental forms — Ember, Bramble, Gale and Tide — each with its own health
pool, attack power and a once-per-battle special. Elements run a four-cycle:
**fire > grass > storm > water > fire**. Read the matchup, manage four health
bars, and fell each foe before all four forms fall. A fresh turn-based battle
RPG genre alongside the other `samples/` pixel games.

## Features

- A 6-foe campaign — Sprout, Gust, Brine, Cinder, Thornlord, Maelstrom — with
  health, power and a charged-attack gimmick climbing across the run.
- Four elemental forms, each its own creature: Ember the glass cannon,
  Bramble the wall, Gale the quick striker, Tide the mender. Striking in a
  super-effective form hits for ×1.6; the same form also resists that foe.
- Real decisions every turn: STRIKE, GUARD (halves the incoming hit and
  empowers your next blow), a once-per-battle SPECIAL, or SHIFT to another
  form. A voluntary shift costs the turn — the foe acts — while a forced shift
  after a knock-out is free.
- Four specials with distinct identities — Pyre (heavy blow, slight recoil),
  Bulwark (heal + guard + empower), Tempo (two strikes), Mend (heal every
  living form).
- Three-star scoring by forms left standing, per-foe stars and progressive
  unlocks, tap or keyboard (1–4 shift, Z/X/C act), English/中文 toggle, saved
  to `localStorage`.
- Verified: 63 checks — the element chart is a clean four-cycle, a competent
  bot clears all 6 foes and out-scores naive play on every foe, strike /
  guard-empower / voluntary vs forced shift / knock-out / defeat and all four
  specials behave; plus a 4-script load-and-render smoke test that wins a
  battle and reads back the save.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4429
```

Then visit `http://127.0.0.1:4429/index.html`.

## Play

- Tap **STRIKE** to attack with your active form — match its element against
  the foe for a super-effective hit.
- Tap a **form chip** to shift; a voluntary shift costs your turn, so spend it
  wisely. After a knock-out the next shift is free.
- Tap **GUARD** before a charged foe attack to halve it and empower your next
  strike. Each form's **SPECIAL** can be used once per battle.
- Fell the foe before all four forms are knocked out. Keep forms alive for
  more stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the element chart, forms, foes, the turn-by-turn battle logic.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, foe creatures, the elemancer, form chips, title art.
- `js/game.js` - screen flow, real-time render loop, input, floating text, save.
