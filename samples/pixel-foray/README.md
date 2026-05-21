# Pixel Foray

A pixel-art **turn-based telegraphed-tactics dungeon**. Every foe shows its
plan before you act — a ghost on the tile a melee foe will step to, a red line
for an archer's shot, red tiles where you would be hit. Then you act once:
move to a neighbour, strike an adjacent foe, or wait. A killing blow cancels a
foe's turn; a mere wound does not. Clear the room. A fresh tactics-dungeon
genre alongside the other `samples/` pixel games.

## Features

- A 6-room dungeon — Threshold, Guardroom, Crossway, Stronghold, Gallery,
  Throne — climbing from two foes to five, with walls for cover.
- Three foe types, each telegraphed: the Grunt steps in and strikes, the
  Brute is a four-health wall that lumbers every other turn, the Archer
  locks a line of fire and forces you behind cover.
- Every turn is a solvable little puzzle: read the red danger tiles and the
  ghost markers, then choose the one move that lands a kill or keeps you
  safe — striking a foe dead cancels exactly the turn it telegraphed.
- Three-star scoring by health left, per-room stars and progressive unlocks,
  restart any time. Tap a tile to move or strike; arrow keys plus Space to
  wait work on desktop. English/中文 toggle, saved to `localStorage`.
- Verified: 69 checks — every room is structurally sound and a depth-7 search
  bot clears all six, telegraph honesty (a foe executes exactly its locked
  plan), strike damage / the Brute's two-hit health / its every-other-turn
  pace / archer fire lines / danger prediction / win and loss all behave;
  plus a 4-script load-and-render smoke test that drives a room to a clear.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4437
```

Then visit `http://127.0.0.1:4437/index.html`.

## Play

- Tap a tile next to your knight to move there; tap an adjacent foe to strike
  it for two damage.
- Before you act, every foe shows its plan — a coloured ghost is where a melee
  foe will step, a red line is an archer's shot. Red tiles will damage you.
- Kill a foe and its telegraphed turn is cancelled; only wounding it is not
  enough. A Brute takes two hits.
- Clear every foe to win the room. Keep your health up for more stars.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the rooms, foe planning, the turn resolution and danger model.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - backdrop, dungeon grid, foes, the telegraph overlay, title art.
- `js/game.js` - screen flow, tap / keyboard input, save.
