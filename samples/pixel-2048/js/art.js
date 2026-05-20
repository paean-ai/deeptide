// Pixel-art rendering for Pixel 2048. 360x480 world units.

const PALETTE = {
  bg:        '#1c1717',
  card:      '#2a2522',
  cardEdge:  '#0e0a08',
  cellEmpty: '#3a3331',
  border:    '#0a0807',
  hudText:   '#fbf7ef',
  hudDim:    '#a89c92',
  highlight: '#f0c570',
  win:       '#f4d27b',
  warn:      '#ff7a7a',
};

// Distinctive colour ramp per tile value (warm low, cool high).
const TILE_COLOURS = {
  2:    { bg: '#eee4da', fg: '#3a3331' },
  4:    { bg: '#ede0c8', fg: '#3a3331' },
  8:    { bg: '#f2b179', fg: '#fff7ed' },
  16:   { bg: '#f59563', fg: '#fff7ed' },
  32:   { bg: '#f67c5f', fg: '#fff7ed' },
  64:   { bg: '#f65e3b', fg: '#fff7ed' },
  128:  { bg: '#edcf72', fg: '#fff7ed' },
  256:  { bg: '#edcc61', fg: '#fff7ed' },
  512:  { bg: '#edc850', fg: '#fff7ed' },
  1024: { bg: '#edc53f', fg: '#fff7ed' },
  2048: { bg: '#edc22e', fg: '#fff7ed' },
};
function tileColour(v) {
  if (TILE_COLOURS[v]) return TILE_COLOURS[v];
  // 4096+ — purple/blue gradient
  if (v <= 8192)  return { bg: '#5a3aa8', fg: '#fff7ed' };
  if (v <= 16384) return { bg: '#3a4ab8', fg: '#fff7ed' };
  return { bg: '#1e2a72', fg: '#fff7ed' };
}

// Board geometry: 4x4 grid centred in a card. Tile cell = 64x64 with
// 8px gaps; card is 296x296 starting at (32, 96).
const BOARD_OX = 32;
const BOARD_OY = 110;
const CELL = 64;
const GAP = 8;
const CARD_W = SIZE * CELL + (SIZE + 1) * GAP;

function cellRect(x, y) {
  return {
    x: BOARD_OX + GAP + x * (CELL + GAP),
    y: BOARD_OY + GAP + y * (CELL + GAP),
    w: CELL, h: CELL,
  };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // Subtle pixel speckle for warmth.
  ctx.fillStyle = '#241d1b';
  for (let i = 0; i < 28; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s) {
  // Card.
  ctx.fillStyle = PALETTE.cardEdge;
  ctx.fillRect(BOARD_OX - 1, BOARD_OY - 1, CARD_W + 2, CARD_W + 2);
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(BOARD_OX, BOARD_OY, CARD_W, CARD_W);
  // Empty cells.
  for (let y = 0; y < SIZE; y++) for (let x = 0; x < SIZE; x++) {
    const r = cellRect(x, y);
    ctx.fillStyle = PALETTE.cellEmpty;
    ctx.fillRect(r.x, r.y, r.w, r.h);
  }
  // Filled tiles.
  for (let y = 0; y < SIZE; y++) for (let x = 0; x < SIZE; x++) {
    const v = s.grid[y][x];
    if (!v) continue;
    drawTile(ctx, cellRect(x, y), v, s.lastSpawn && s.lastSpawn.x === x && s.lastSpawn.y === y);
  }
}

function drawTile(ctx, r, v, fresh) {
  const c = tileColour(v);
  // Pixel-art double-bevel: dark base, light top-edge.
  ctx.fillStyle = PALETTE.border;
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = c.bg;
  ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
  // Top bevel
  const hi = lighten(c.bg, 0.18);
  ctx.fillStyle = hi;
  ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, 3);
  ctx.fillRect(r.x + 1, r.y + 1, 3, r.h - 2);
  // Bottom-shadow
  const lo = darken(c.bg, 0.22);
  ctx.fillStyle = lo;
  ctx.fillRect(r.x + 1, r.y + r.h - 4, r.w - 2, 3);
  ctx.fillRect(r.x + r.w - 4, r.y + 1, 3, r.h - 2);
  if (fresh) {
    ctx.fillStyle = 'rgba(255,255,255,0.18)';
    ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
  }
  // Value label — chunky bitmap-style font (canvas builtin in monospace).
  ctx.fillStyle = c.fg;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const str = String(v);
  const size = str.length <= 2 ? 26 : str.length === 3 ? 22 : str.length === 4 ? 18 : 14;
  ctx.font = `bold ${size}px monospace`;
  ctx.fillText(str, r.x + r.w / 2, r.y + r.h / 2 + 1);
  ctx.textBaseline = 'alphabetic';
}

// Approximate light/dark tint over a hex colour without external libs.
function lighten(hex, amt) {
  const { r, g, b } = parseHex(hex);
  return rgbToHex(clamp(r + 255 * amt), clamp(g + 255 * amt), clamp(b + 255 * amt));
}
function darken(hex, amt) {
  const { r, g, b } = parseHex(hex);
  return rgbToHex(clamp(r - 255 * amt), clamp(g - 255 * amt), clamp(b - 255 * amt));
}
function clamp(v) { return Math.max(0, Math.min(255, Math.round(v))); }
function parseHex(h) {
  const n = parseInt(h.slice(1), 16);
  return { r: (n >> 16) & 0xff, g: (n >> 8) & 0xff, b: n & 0xff };
}
function rgbToHex(r, g, b) { return '#' + [r, g, b].map(v => v.toString(16).padStart(2, '0')).join(''); }

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.cardEdge;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'title'), 8, 16);
  // Score + best as chips just under the HUD strip.
  drawChip(ctx, 24,  44, 144, 40, t(lang, 'score'), String(s.score), PALETTE.highlight);
  drawChip(ctx, 192, 44, 144, 40, t(lang, 'high'),  String(best),    PALETTE.hudDim);
}

function drawChip(ctx, x, y, w, h, label, value, tint) {
  ctx.fillStyle = PALETTE.cardEdge;
  ctx.fillRect(x - 1, y - 1, w + 2, h + 2);
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = tint;
  ctx.fillRect(x, y, w, 3);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = 'bold 10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label.toUpperCase(), x + w / 2, y + 12);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 18px monospace';
  ctx.fillText(value, x + w / 2, y + 28);
  ctx.textBaseline = 'alphabetic';
}
