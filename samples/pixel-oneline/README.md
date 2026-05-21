# Pixel One-Line

A pixel-art **one-stroke drawing** puzzle (Chinese 一笔画) — trace a
single continuous line that covers every edge of the figure exactly
once, never lifting the pen. A fresh graph-tracing puzzle alongside
the other `samples/` pixel games.

## Features

- 6-figure campaign **Envelope → Fan** — the classic envelope, an
  hourglass, the five-point star, a 3-square lattice, a battlement
  crown and a five-spoke fan.
- Every figure is a graph that admits an **Eulerian path** — it is
  connected and has either 0 or 2 odd-degree vertices. The test
  harness checks that property AND runs a depth-first solver to
  confirm a full one-stroke trail genuinely exists.
- Tap a node to place the pen, then tap a connected node to draw
  that edge; the reachable nodes glow pink so the next move always
  reads. Each edge can be traced only once.
- Unlimited **Undo** (steps back one edge, or lifts the pen on the
  first node) and **Restart** (`Z` / `R` on desktop).
- Level select with progressive unlocks; cleared figures saved to
  `localStorage`.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 45 mechanics checks — every figure is connected with a
  valid 0-or-2 odd-vertex count and no duplicate edges; an
  exhaustive DFS confirms each is solvable; tap places the pen and
  traverses real edges only; a used edge cannot be re-traced; undo
  frees the last edge and moves the pen back (or lifts it); restart
  wipes state; playing a full DFS-found trail triggers the win.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4336
```

Then visit `http://127.0.0.1:4336/index.html`.

## Play

- Tap a node to place the pen.
- Tap a node joined to the pen by an un-drawn edge to draw it; the
  pink nodes show where you can go next.
- Cover every edge exactly once in one continuous line to win.
  Use **Undo** to back out of a dead end.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — figure graphs (nodes + edges), degree / connectivity /
  Eulerian-path helpers, trace + undo + restart state.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, edges (drawn / undrawn with glow), nodes
  (pen / reachable / plain), HUD, stars.
- `js/game.js` — screen flow, tap input, reachable-set highlight, save.
