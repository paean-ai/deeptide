# Responsive Canvas Game Shell

Use this skill when building a dependency-free HTML Canvas game that must work
on desktop and mobile with stable layout, crisp rendering, and reliable input.

## Goal

Create a game shell that makes rendering, input, UI, and verification predictable
before complex gameplay is added.

## Recommended Structure

```text
sample-game/
  index.html
  css/style.css
  js/data.js
  js/assets.js
  js/game.js
  README.md
```

Use fewer files for tiny samples, but keep data, art helpers, and main loop
separable when the game has more than one mechanic.

## HTML Shell

```html
<div id="game-wrapper">
  <canvas id="gameCanvas"></canvas>
  <div id="hud">...</div>
  <div id="modal" class="hidden">...</div>
  <div id="touch-controls">...</div>
</div>
```

The canvas should be the primary surface. HUD and panels should be positioned
around it, not randomly layered over critical gameplay.

## Canvas Resize

Use CSS for visual size and device-independent canvas pixels for drawing:

```js
function resizeCanvas() {
  const rect = canvas.getBoundingClientRect();
  canvas.width = Math.max(320, Math.floor(rect.width));
  canvas.height = Math.max(320, Math.floor(rect.height));
  ctx.imageSmoothingEnabled = false;
}

window.addEventListener('resize', resizeCanvas);
window.addEventListener('orientationchange', resizeCanvas);
resizeCanvas();
```

For pixel art, avoid device-pixel-ratio scaling unless the whole renderer is
designed for it. Consistency is more important than theoretical sharpness.

## Layout CSS

```css
body {
  margin: 0;
  min-height: 100dvh;
  display: grid;
  place-items: center;
  overflow: hidden;
  user-select: none;
}

#game-wrapper {
  position: relative;
  width: 900px;
  aspect-ratio: 900 / 700;
  max-width: 100vw;
  max-height: 100dvh;
  overflow: hidden;
}

canvas {
  display: block;
  width: 100%;
  height: 100%;
  image-rendering: pixelated;
  touch-action: none;
}
```

On mobile, allow scrolling only when the game is not full-screen or when the
sample intentionally becomes a document-like layout.

## Input Rules

- Normalize pointer coordinates from `getBoundingClientRect`.
- Use pointer events for shared mouse/touch behavior.
- Add `touch-action: none` to active canvas/board regions.
- Add drag thresholds to prevent tap/drag conflicts.
- Handle `pointercancel`.
- Keep mobile buttons large and fixed-size.

Example:

```js
function pointerPos(ev) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (ev.clientX - rect.left) * canvas.width / rect.width,
    y: (ev.clientY - rect.top) * canvas.height / rect.height,
  };
}
```

## Main Loop

```js
let last = 0;

function loop(time) {
  const dt = Math.min(2.4, (time - last) / 16.67 || 1);
  last = time;
  update(dt);
  render();
  requestAnimationFrame(loop);
}

requestAnimationFrame(loop);
```

Clamp `dt` so tab switching or mobile throttling does not explode physics.

## UI State

Use explicit states:

```js
const state = {
  mode: 'build', // title | playing | build | combat | reward | gameover
  wave: 1,
  score: 0,
  selected: null,
  messages: [],
};
```

Keep rendering functions pure where possible:

- `renderWorld()`
- `renderActors()`
- `renderEffects()`
- `renderHUD()`
- `renderPanels()`

## Verification Checklist

- Open directly in a browser if no module imports are used.
- Serve locally if relative module paths or fetches are involved.
- Run syntax checks for all JS files.
- Capture desktop and mobile screenshots.
- Confirm the canvas is nonblank.
- Confirm touch controls or pointer interactions work on mobile width.
- Confirm no text overlaps or horizontal overflow.
- Confirm game-over/restart and modal states do not trap input.

## Common Problems

- Canvas CSS size and drawing size diverge after resize.
- HUD overlays hide important enemies or controls.
- Mobile Safari scrolls while dragging.
- Text in buttons overflows at 390px width.
- `setInterval` game loops drift and continue after hidden tabs.
- Particles and arrays grow forever.
- Dead entities are not filtered, causing performance decay.
