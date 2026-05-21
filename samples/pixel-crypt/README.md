# Pixel Crypt

A pixel-art **Mamono-Sweeper**-style RPG grid crawler. Every tile of
the crypt hides a monster or is empty. Reveal an empty tile and it
shows the **sum of the monster levels around it**; reveal a monster
and you fight it — a foe at or below your level falls for free, a
stronger one wounds you by the gap. Slay weak foes to level up, then
take the rest. A fresh deduction-RPG genre alongside the other
`samples/` pixel games.

## Features

- 6-floor campaign **Cellar → Throne** on growing grids (6×6 → 9×9)
  with deeper monster mixes (level 1 up to level 5).
- Minesweeper-style deduction with an RPG twist: an empty tile's
  number is the **level-sum** of its eight neighbours, so a clue of 5
  could mean one level-5 brute or a level-2 and a level-3 — read the
  board to find safe prey.
- A real progression loop: every monster slain grants XP equal to its
  level; crossing an XP threshold raises your hero level, and a higher
  level means more foes you can take unscathed.
- 20 HP. Fight a monster above your level and you lose HP equal to the
  gap — clear the level-1s first, level up, and the deep foes become
  free.
- A revealed empty tile with no monsters around it **floods open** its
  neighbours, just like the classic.
- A **flag mode** toggle to mark suspected lairs while you reason.
- Per-floor best score (HP × 5 + XP) and progressive unlocks, saved to
  `localStorage`.
- HUD with an HP bar, hero level and XP-to-next bar, and a slain count.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, tap controls
  tuned for touch.
- Verified: 49 checks — `heroLevel` follows the XP thresholds; every
  floor places exactly its monster mix with the right level spread
  and a clue equal to the neighbour level-sum, and is deterministic;
  a 0-clue tile floods its neighbours; an over-level monster wounds
  the hero by the level gap while an equal-or-weaker one is free;
  flags toggle and protect a tile from reveal; clearing every monster
  in ascending order wins each floor unscathed; lethal damage ends
  the run as a loss; plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4405
```

Then visit `http://127.0.0.1:4405/index.html`.

## Play

- Tap an **empty tile** — it reveals the sum of the monster levels in
  the eight tiles around it.
- Tap a **monster** to fight it. At or below your level it falls for
  free; above your level it costs HP equal to the gap.
- Every kill is XP — slay the weak foes first to level up, then the
  strong ones become safe.
- Switch to **flag mode** to mark suspected monster tiles.
- Slay every monster to clear the floor; reach 0 HP and you fall.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — monster placement, level-sum clues, fight / XP /
  level-up / HP rules, flood reveal, win-loss logic.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, raised / sunken tiles, monsters by level,
  clue numbers, HUD with HP and XP bars.
- `js/game.js` — screen flow, tap input, reveal / flag modes, save.
