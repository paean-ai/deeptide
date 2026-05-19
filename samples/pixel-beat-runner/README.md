# Pixel Beat Runner

A dependency-free 4-lane rhythm game — tap each lane as its note crosses the
judgement line. Every hit plays a procedural Web Audio tone, so a clean run
*is* the melody. A fresh rhythm genre alongside the other `samples/` games.

## Features

- 4-lane note highway with falling notes and timing judgement
  (Perfect / Good / Miss).
- Procedural sound via the Web Audio API — each lane is a C-major pitch
  (C E G C); no audio files, no dependencies.
- Combo scoring with a rising multiplier, live accuracy, and a health bar that
  drains on misses — out of sync and the run ends.
- Endless procedurally charted beats whose tempo ramps from 100 up to 188 BPM.
- Struck-lane flashes and hit-burst effects.
- `localStorage` best-score record.
- English / 中文 toggle.
- Responsive portrait canvas: keys `D F J K`, on-screen lane buttons, or tap
  the lane directly on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4201
```

Then visit `http://127.0.0.1:4201/index.html`.

## Play

- A note falls down each lane; hit that lane the moment it reaches the line.
- **Desktop:** `D F J K` for the four lanes, `Esc` to pause.
- **Mobile:** tap the lane on the highway, or use the four lane buttons.
- Dead-centre timing is a Perfect; close is a Good; late or early is a Miss.
- Misses drain your health bar — chase a long combo and high accuracy.

## Structure

- `index.html` - title / game screens, HUD, lane buttons, overlays.
- `css/style.css` - responsive rhythm UI, lane buttons.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - lanes, pitches, timing windows, tempo tuning.
- `js/art.js` - lane highway, notes, hit effects.
- `js/game.js` - charting, judgement, Web Audio tones, screens, loop.
