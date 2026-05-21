# Pixel Black Box

A pixel-art take on the classic **Black Box** deduction puzzle (Eric
Solomon, 1976; popularised by an Atari version). Hidden atoms sit on a
grid; fire probes in from any edge and read the result — a HIT, a
REFLECT, or a labelled PASS-THROUGH that pairs an entry edge with an
exit edge — then deduce where the atoms must lie. A fresh deduction
genre alongside the other `samples/` pixel games.

## Features

- 9-puzzle campaign **Atom → Event Horizon** scaling grid size 6 → 9
  and atom count 3 → 7, all stored as a seeded layout so each level is
  reproducible. The 9×9 **Cosmos, Void and Event Horizon** boards
  give the most probe routes to reason over.
- Faithful Black Box ray simulation: HIT when the next cell holds an
  atom; **edge-reflect** when the entry cell already has an atom in
  the perpendicular-side cell; **deflect 90°** away from a single
  perpendicular atom; **bounce-back-reflect** when atoms sit on both
  perpendicular sides.
- Pass-through entry / exit edges share an auto-assigned letter label
  (A, B, C…) so you can trace the ray's route at a glance.
- Tap an edge cell to fire a probe. Tap an interior cell to mark
  (or un-mark) a suspected atom. **Reveal** scores 200 per correct
  mark, -100 per incorrect mark, -5 per probe used.
- Level select with progressive unlocks; per-puzzle best score saved
  to `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 75 checks — all 9 levels place the right number of
  distinct atoms, all on interior cells (so probes can't trivially
  edge-reflect every shot); every grid (including the new 9×9 boards)
  fits the 360×480 frame; the generator is deterministic; every edge
  probe resolves to a result; plus a load check that all four scripts
  run cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4322
```

Then visit `http://127.0.0.1:4322/index.html`.

## Play

- Tap any edge cell (the dotted ring around the grid) to fire a probe
  inward. Result lands on the same edge as a letter / `H` / `R`.
- Tap an interior cell to mark / un-mark a suspected atom.
- When you're confident, tap **Reveal** to score.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — edge-index math, atom placement, classic Black Box
  ray simulation, mark/reveal/scoring state.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, grid + edge-button strip, atom render,
  mark / hit / pass labels, HUD.
- `js/game.js` — screen flow, tap input, save.
