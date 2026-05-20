# Pixel Hanoi

A pixel-art **Tower of Hanoi** — move every disk from the left peg to
the right peg one at a time, never placing a larger disk on top of a
smaller one. Solve in the optimal **2^N - 1** moves to earn three
stars. A fresh classic-puzzle sample alongside the others in `samples/`.

## Features

- 6-level campaign **Spire → Colossus** with 3 → 8 disks (pars
  7 / 15 / 31 / 63 / 127 / 255 moves).
- Faithful Tower of Hanoi mechanics: pick up the top disk of any peg,
  drop it onto a peg whose top disk is larger (or onto an empty peg);
  illegal stacks are rejected.
- **Tap-then-tap** input: tap a peg to pick up its top disk, tap
  another peg to drop. Re-tap the same peg to cancel a selection. On
  desktop, **1 / 2 / 3** select pegs and **Z / R** undo / restart.
- Unlimited **Undo** and **Restart** at any time.
- Star rating per level: 3 = optimal 2^N - 1, 2 = within +50 %,
  1 = solved-but-loose. Per-level **lowest-move** record persisted
  to `localStorage`.
- Eight-colour disk ramp (red / orange / yellow / green / blue /
  purple / pink / lavender) so every disk reads at a glance.
- English / 中文 toggle.
- 360 × 480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 97 mechanics checks — par formula, every level builds
  with correct stack ordering and an empty middle / right peg,
  every legal and illegal move enforced (larger-on-smaller rejected,
  empty-peg-source ignored), tap-then-tap routes through `tapPeg`
  correctly, undo round-trip, restart wipe, and a full recursive
  optimal-solution generator applied to every level yields exactly
  the par count and lands the game solved.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4326
```

Then visit `http://127.0.0.1:4326/index.html`.

## Play

- Tap a peg to **pick up** its top disk. Tap another peg to **drop**.
  Re-tap the same peg to cancel.
- Never drop a larger disk on top of a smaller one.
- Move every disk to the right peg. Match par (**2^N - 1**) for 3 stars.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — pegs / disks model, legal-move rules, tap-then-tap
  state, undo stack, recursive optimal-solution generator.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, wooden ground + brass pegs, tapered disk
  ramp with peg-through highlight, lifted-disk selection ring, HUD.
- `js/game.js` — screen flow, tap input, save.
