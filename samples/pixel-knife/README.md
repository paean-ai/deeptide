# Pixel Knife

A pixel-art take on the mobile classic **knife-hit** — a wooden disk spins
and you tap to launch a knife from below. The knife sticks where it meets
the rim; if it lands on a knife that's already there, it snaps. Land every
knife in the log to clear the round. A fresh tap-only arcade entry
alongside the other `samples/` pixel games.

## Features

- 6-log campaign **Sapling → Ironwood** — each log raises the knife
  quota (5 → 10), the rotation speed (1.05 → 3.20 rad/s), the rotation
  **pattern**, and from L3 onward sprinkles **apple** bonus targets on
  the disk that score +50 when struck.
- Three rotation patterns:
  - **steady** — constant CW spin.
  - **reverse** — direction flips every 1.6 s.
  - **pulse** — short 0.5 s bursts at ~2× speed every 1.5 s.
- Pre-pinned starter knives at each log so the very first throw is a real
  timing puzzle, not an empty disk.
- One knife in flight at a time — tap-only input fits any phone screen
  with no gestures or drag. Keyboard fallback: **space** / **enter**.
- Disk-local-angle landing math: a flying knife lands at `rel = -disk.angle`
  in the disk's rotating frame; any existing knife within 0.21 rad
  (~12°) snaps yours. Apples use an 0.18 rad pickup window.
- Score = `10 × stuck + 50 × apples`. Per-log best score saved.
- English / 中文, `localStorage` save with cleared logs and best scores.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 38 mechanics checks — flying-knife flight, queue counter,
  no-double-throw guard, forced-overlap fails, forced-clean-stick wins,
  forced 5-evenly-spaced-throws wins L1, apple absorbs without sticking,
  every level builds with sane config, angular distance correct.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4299
```

Then visit `http://127.0.0.1:4299/index.html`.

## Play

- Tap anywhere to throw the next knife straight up.
- Wait for a clear slice of the spinning disk before you tap.
- Apples are bonus — landing on one **absorbs the throw** (no knife stuck)
  but pays +50 score.
- Clear all knives in the log to win. Two knives meeting = snap.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — disk physics, three rotation patterns, throw / land math,
  apple pickup, win / fail detection.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, wood disk with grain rings, knife sprite, apple
  sprite, queue indicator, HUD.
- `js/game.js` — screen flow, tap + keyboard input,
  requestAnimationFrame loop, save.
