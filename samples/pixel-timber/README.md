# Pixel Timber

A pixel-art **chop-and-dodge** arcade. The tree trunk is a stack of
logs and you are the lumberjack standing beside it. Every tap fells the
bottom log — the stack drops a notch, a fresh log appears on top, and
the score ticks up. But some logs carry a branch: if the new bottom
log's branch is on the side you're standing, it knocks you flat. A
stamina bar drains the whole time, so you can never stop swinging. A
fresh reaction-arcade genre alongside the other `samples/` pixel games.

## Features

- One-thumb endless arcade — tap the **left or right half** of the
  screen (or ◀ ▶ / A·D) to step to that side and chop.
- Every chop fells the bottom log, drops the stack, and procedurally
  generates a new log on top. Each log has at most one branch, so
  there is **always a safe side** — the challenge is reading the next
  log in time, not luck.
- A stamina bar that drains continuously and **faster as your score
  climbs**; each chop tops it up, so a steady rhythm is what keeps you
  alive — pausing to think is itself a risk.
- Two ways to fall: a branch catches you, or your stamina runs dry —
  the result screen tells you which.
- Juicy feedback — an axe-swing pose, the felled log tumbling off
  screen, and a screen-shake on the knock-out hit.
- Chunky pixel art: a bark-textured trunk with end-grain rings on the
  log next to be cut, leafy branches, a parallax forest backdrop.
- Best score saved to `localStorage`.
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first
  tap-anywhere controls.
- Verified: 25 checks — the trunk builds with the bottom two logs
  clear; a chop scores, refills stamina, keeps the stack length
  stable and spawns the flying log; a branch on your side ends the
  run while the opposite side is safe; stamina depletion ends the run
  flagged as a time-out; chopping after game-over is ignored; the
  generator is deterministic per seed; a perfect-dodge bot survives
  400 chops across 40 seeds (proving every log leaves a safe side);
  plus a UI smoke pass.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4365
```

Then visit `http://127.0.0.1:4365/index.html`.

## Play

- Tap the **left** or **right** side of the screen to chop the bottom
  log — you step to that side as you swing.
- Watch the log **second from the bottom**: that's the one about to
  drop onto you. If its branch points your way, chop from the other
  side.
- Keep chopping — the stamina bar never stops draining, and only a
  chop refills it.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — trunk stack, branch generation, chop / dodge
  resolution, draining-stamina model, scoring.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, parallax forest, bark-textured trunk with
  branches, the lumberjack with an axe-swing pose, HUD + stamina bar.
- `js/game.js` — screen flow, tap / key input, RAF loop, save.
