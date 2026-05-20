# Pixel Bowl

A pixel-art **10-pin bowling** game with a single top-down lane and proper
traditional scoring. Drag from the ball in the direction you want it to
roll, release to bowl. A fresh swipe-only arcade entry alongside the
other `samples/` pixel games.

## Features

- 10 frames of classic bowling, two throws each, with a third throw in
  frame 10 on a strike or spare — running-total score uses real
  strike-bonus (10 + next two) and spare-bonus (10 + next one).
- Swipe physics: drag UP from the ball to set both direction and power,
  then release. Curve the swipe to put english on the ball. Drag must
  travel up-the-lane or it's rejected.
- Pin chain reaction: the ball deflects off the first pin it hits, and
  any standing pin within `FALL_RADIUS` of a fallen pin tips over too —
  good aim at the head pin can still send the whole rack flying.
- Compact 10-frame score strip across the top of the screen; each frame
  shows its throws (`X` strike, `/` spare, digit otherwise) and the
  running total once the bonus is settled.
- Persistent **best-score** save (perfect game = 300).
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 17 mechanics checks — fresh game shape, downward-swipe
  rejected, upward swipe fires the ball, ball resolves within guard,
  strong centre throw knocks ≥8 pins, head pin has ≥2 neighbours within
  `FALL_RADIUS`, plus the four scoring cases (open 5+3=8, spare
  7+3+4=14, strike 10+4+3=17, perfect game = 300).

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4301
```

Then visit `http://127.0.0.1:4301/index.html`.

## Play

- Drag from the ball **upward** to aim — the yellow dotted guide shows
  where you're rolling and the dot turns red as you near max power.
- Release to roll. The ball deflects off the first pin it hits and any
  pin that falls topples its near neighbours.
- 10 frames, 2 throws each. Knock all 10 pins on the first throw for a
  **strike** (`X`), or finish them on the second for a **spare** (`/`).
- Frame 10 grants a third throw if you score a strike or spare in it.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — lane physics (friction + gutter bounces), pin layout,
  ball-pin contact + cascade-knock, traditional scoring with bonuses.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, lane planks, pin sprites (standing + fallen),
  ball with finger holes, dotted aim guide, frame-by-frame score strip.
- `js/game.js` — screen flow, drag input, requestAnimationFrame loop,
  save.
