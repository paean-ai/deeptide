# Pixel Curl

A pixel-art **curling** arcade. Slide stones up an icy sheet toward the
target circle (the "house"); an AI opponent throws stones too, alternating
turns. After all eight stones are thrown, you score one point for every
stone of yours that's closer to the centre than the opponent's best.
A fresh turn-based physics entry alongside the other `samples/` pixel
games.

## Features

- 9-match campaign **Rookie → Flawless** with shrinking AI aim sigma
  (32 → 3 px Gaussian noise on the target landing point) — the
  Legend, Grand Slam and Flawless opponents land almost every stone
  right on the button.
- Real ball physics — friction (`vel *= 0.32^dt`), gutter / sideline
  bounce (60 % retained), pairwise **elastic** ball-ball collisions
  with overlap correction, 240 Hz substepping inside the variable-dt
  tick so fast knockouts don't tunnel.
- Drag-AWAY slingshot input: pull back from the spawn pad at the
  bottom centre, release to slide forward (up the sheet). Drag must
  be a meaningful downward pull (`dy ≥ 10`) — otherwise the throw is
  rejected. Dotted yellow trail behind the stone shows pull-back,
  brighter dotted line ahead previews the slide; end dot turns red
  near max power.
- Alternating turns: after each of your stones stops, the AI throws
  after a brief delay, then it's your turn again. Stones-remaining
  pips at the top of the HUD show how many you each have left.
- Standard end scoring: stones inside the house are sorted by distance
  from the centre; the side owning the closest stone earns 1 point per
  consecutive stone closer than the opponent's nearest.
- AI throws are calibrated against the friction model — `v0 = dist
  × -ln(FRICTION) × 1.05` aims at the house centre + Gaussian noise
  scaled by the level's `aiSigma`.
- Per-match best-score save + a clear gate (winning the end unlocks
  the next opponent).
- English / 中文 toggle.
- 360×480 responsive frame, `image-rendering: pixelated`, mobile-first.
- Verified: 29 checks — all 9 matches build with their AI sigma, the
  sigma only ever tightens across the campaign, and every sigma is
  sane; plus a load check that all four scripts run cleanly.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4313
```

Then visit `http://127.0.0.1:4313/index.html`.

## Play

- Drag DOWN from the red stone — pull-back trail shows power, brighter
  dotted line ahead shows the slide; release to launch.
- Curl-style scoring: only stones inside the house count; you win the
  end with however many of your stones beat the opponent's nearest.
- Knocking the opponent's stones out is fair game; angled bounces help.

## Structure

- `index.html` — shell + script tags.
- `css/style.css` — 360:480 responsive frame, `image-rendering: pixelated`.
- `js/data.js` — ice-sheet physics (friction + bounce + elastic
  collisions), 240 Hz substep, alternating-turn pump, AI throw
  calibration, end-scoring rule.
- `js/i18n.js` — English / Chinese strings.
- `js/art.js` — palette, ice sheet with centre + hog lines, house with
  4 concentric rings, stone sprites (red player / blue AI), aim guide,
  stone-spawn pad ring, HUD with stones-remaining pips.
- `js/game.js` — screen flow, drag input,
  requestAnimationFrame loop, save.
