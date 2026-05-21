# Pixel Untangle

A pixel-art take on the classic **Planarity / Untangle** puzzle. A graph
of pegs joined by threads starts as a tangled knot — drag the pegs
around until **no two threads cross**. A fresh graph-puzzle genre
alongside the other `samples/` pixel games.

## Features

- 6-level campaign **Knot → Gordian** on growing graphs (6 → 16 pegs,
  ~2 threads per peg).
- Every puzzle is built **from a known crossing-free layout** — the
  generator scatters pegs, adds the shortest threads that do not cross
  one already placed, then scrambles the pegs into a tangle. Because
  the clean layout exists by construction, **a solution is always
  guaranteed**.
- Live crossing detection: any thread currently crossing another is
  drawn **red**, and the HUD counts the crossings as you drag — solve
  it the moment the count hits zero.
- Drag any peg with a generous touch radius; the threads follow.
- Score = `999 - seconds` on a clean solve; level select with
  progressive unlocks and a per-level best, saved to `localStorage`.
- Peg-board pixel art — studded pegs, outlined threads, a grained mat.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, drag controls
  tuned for touch.
- Verified: 67 checks — segment-crossing geometry (a true crossing, a
  near-miss, and a shared endpoint); every level builds with valid,
  unique, self-loop-free edges and no dangling pegs; the reference
  layout is crossing-free so a solution always exists; the scrambled
  start is genuinely tangled and never pre-solved; the crossing-set
  indices are valid; pegs sit inside the board; the generator is
  deterministic; plus a UI smoke pass that drags a peg.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4369
```

Then visit `http://127.0.0.1:4369/index.html`.

## Play

- Drag a **peg** to move it — the threads tied to it move too.
- A **red** thread is crossing another thread; a teal thread is clear.
- Rearrange the pegs until the crossing count reaches **zero**.
- Every puzzle is solvable — there is no dead end, only a tangle.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — peg scatter, non-crossing thread generation, the
  scramble, segment-crossing geometry and the solved check.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, peg board, outlined threads (red when
  crossing), studded pegs, HUD.
- `js/game.js` — screen flow, drag input, timer, save.
