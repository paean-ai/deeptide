# Pixel Art With Code

Use this skill when creating pixel-art visuals directly in code: Canvas sprites,
matrix glyphs, CSS pixel UI, item icons, tilemaps, particles, and retro game
effects. It is derived from the repository's pixel roguelike, dungeon crawler,
platformer, idle, and backpack samples.

## Goal

Produce pixel art that feels intentionally designed rather than merely
low-resolution. The result should be readable at gameplay speed, consistent
across assets, and easy for future agents to edit.

## Core Principles

- **Author the pixel grid explicitly.** Prefer sprite matrices, small SVGs, or
  deterministic Canvas drawing over blurry raster assets when the asset is part
  of the game's core vocabulary.
- **Use a constrained palette.** Define semantic colors once: outline, shadow,
  highlight, metal, health, magic, poison, fire, frost, gold, foliage, stone.
- **Outline first, light second.** Pixel art reads best when silhouettes are
  strong before highlights are added.
- **Preserve hard edges.** Set `image-rendering: pixelated` for rendered
  sprites, canvases, and icon surfaces. Disable `ctx.imageSmoothingEnabled`.
- **Animate by changing shapes, not by tweening blur.** Use 2-3 readable frames,
  bobbing, hit flashes, palette swaps, particles, and screen shake.
- **Design for scale.** A 12x12 sprite should still read when multiplied by 3,
  when viewed on a phone, and when surrounded by combat effects.

## Recommended Asset Structure

For matrix sprites:

```js
const PALETTE = {
  K: '#11131a', // outline
  W: '#f3f7ff', // highlight
  S: '#c4c9d1', // steel
  B: '#2f80ed', // blue cloth
  R: '#e05243', // damage red
  G: '#43d17a', // nature green
  Y: '#f2c14e', // gold
};

const PLAYER_IDLE = [
  '...KKKKK....',
  '..KSSYSSK...',
  '..KSYYYK....',
  '.KBBBBBBK...',
  '.KBRRRBBK...',
  'KBBRRRRBBK..',
  'KBBSSSSBBK..',
  '.KBBSSBBK...',
  '..KBBBBK....',
  '..KS..SK....',
  '.KSS..SSK...',
  '............',
];
```

Render with a helper that maps characters to filled rectangles:

```js
function drawPixelSprite(ctx, frame, palette, x, y, scale = 3) {
  const rows = frame.length;
  const cols = frame[0].length;
  const ox = Math.round(x - cols * scale / 2);
  const oy = Math.round(y - rows * scale / 2);
  ctx.imageSmoothingEnabled = false;

  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const key = frame[row][col];
      if (key !== '.' && palette[key]) {
        ctx.fillStyle = palette[key];
        ctx.fillRect(ox + col * scale, oy + row * scale, scale, scale);
      }
    }
  }
}
```

For small item icons, SVG can be appropriate if it preserves pixel geometry:

```js
function itemGlyph(color) {
  return `
    <svg viewBox="0 0 32 32" class="item-glyph" aria-hidden="true">
      <rect x="14" y="4" width="4" height="18" fill="#f8fbff"/>
      <rect x="12" y="20" width="8" height="4" fill="#0a0d13"/>
      <rect x="15" y="24" width="2" height="6" fill="${color}"/>
    </svg>
  `;
}
```

## Visual Systems To Include

- **Sprite registry:** `SPRITES.player`, `SPRITES.slime`, `SPRITES.boss`, etc.
  with `palette`, `scale`, `anchorY`, and `animations`.
- **Palette modes:** `flash`, `frozen`, `burn`, `poison`, `elite` as palette
  transforms rather than separate sprite files.
- **Pixel shadows:** small rectangular ellipses under actors for grounding.
- **Combat readability:** damage numbers, hit flash, short rings, slash arcs,
  projectile trails, and small particles.
- **Tile variation:** deterministic terrain noise, corner details, cracks,
  grass tufts, stains, or floor glyphs to avoid flat fields.
- **UI parity:** cards, HUD bars, buttons, and inventory cells should use the
  same border, highlight, and shadow language as the sprites.

## Quality Checklist

- The silhouette is readable in grayscale.
- The sprite uses no more colors than necessary.
- The outline is consistent with other assets.
- The asset can be mirrored without breaking key details.
- Hit/burn/frozen states are supported.
- It renders crisply at desktop and mobile sizes.
- The asset is data-driven and easy to modify.

## Common Mistakes

- Using gradients inside tiny sprites.
- Making every sprite the same hue family.
- Over-detailing faces and weapons until the silhouette disappears.
- Drawing UI in a modern flat style while sprites are pixel-art.
- Scaling canvas with smoothing enabled.
- Creating one-off icons without a shared palette or glyph helper.
