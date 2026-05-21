# Pixel Harpoon

A pixel-art **Pang / Buster Bros**-style bouncing-orb shooter. Orbs
bounce around the cavern under gravity — fire a harpoon straight up to
split them. Every hit halves an orb into two smaller ones; the smallest
just pop. Clear the cavern without letting an orb touch you. A fresh
arcade genre alongside the other `samples/` pixel games.

## Features

- 6-stage campaign **Drop In → Onslaught** — escalating orb counts and
  sizes, from a single mid orb to a four-orb onslaught of giants.
- Four orb sizes, each with its own radius, **characteristic bounce
  height** (bigger orbs bounce higher) and score. A hit splits an orb
  into two of the next size down; the tiniest orbs pop outright.
- One harpoon at a time: it climbs as a wire and catches the first orb
  in its column, so positioning under a bouncing orb is the whole
  skill — chain splits, but don't get cornered by what you create.
- 3 lives; an orb that touches you costs one and grants a brief
  invulnerability so you can escape the scrum.
- Three-button control pad — hold **◀ ▶** to move, tap **▲** to fire —
  plus arrow keys / A·D and Space on desktop.
- Per-stage best score (with a lives bonus) and progressive unlocks,
  saved to `localStorage`.
- Chunky pixel art — shaded orbs, a glinting harpoon wire, a speckled
  cavern.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, a touch
  control pad below the play field.
- Verified: 34 checks — every stage spawns its orb set; the harpoon
  fires one at a time; gravity, floor-bounce and wall-bounce behave; a
  hit splits a big orb into two of the next size down and pops the
  smallest with no children; a missed harpoon retracts at the ceiling;
  the player is clamped to the arena; an orb hit costs a life, four
  hits end the run, and invulnerability blocks an instant second hit;
  plus a UI smoke pass with pad input.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4373
```

Then visit `http://127.0.0.1:4373/index.html`.

## Play

- Hold **◀ ▶** (or arrow keys / A·D) to move along the floor.
- Tap **▲** (or Space) to fire the harpoon straight up.
- The harpoon catches the first orb above you and splits it in two —
  the smallest orbs pop for good.
- Keep clear of the bouncing orbs and pop every one to clear the stage.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — orb physics (gravity, bounce, split), the harpoon, the
  player, collisions, 6 stage layouts, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, cavern, shaded orbs, harpoon wire, the diver,
  HUD.
- `js/game.js` — screen flow, control pad + keyboard input, RAF loop,
  save.
