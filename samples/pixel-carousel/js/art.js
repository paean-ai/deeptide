// Pixel Carousel - all canvas drawing: the board, target preview, title.

const PAL = {
  bg0: '#171426', bg1: '#241d38', frame: '#0c0a16',
  panel: '#2b2440', panelHi: '#3d3358', ink: '#0c0a16',
  text: '#f3f1e6', dim: '#998fae', good: '#7bd88f', star: '#ffe27a',
};
const TILE = [
  { base: '#e8554f', hi: '#ff8f88' },
  { base: '#f2a83e', hi: '#ffca78' },
  { base: '#f2d34a', hi: '#fff08a' },
  { base: '#5fc06e', hi: '#9be88a' },
  { base: '#4a9be8', hi: '#8fc8ff' },
  { base: '#9a6cd8', hi: '#c39bf2' },
];

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// ---- layout --------------------------------------------------------------
function gridGeom(cols, rows) {
  const cell = Math.min(54, Math.floor(288 / Math.max(cols, rows)));
  const bw = cell * cols, bh = cell * rows;
  return { cell, bw, bh, ox: (360 - bw) / 2 | 0, oy: 84 + ((290 - bh) / 2 | 0) };
}
const UNDO_BTN = { x: 44, y: 396, w: 130, h: 40 };
const RESTART_BTN = { x: 186, y: 396, w: 130, h: 40 };

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

function drawTile(ctx, x, y, sz, colorIdx) {
  const t = TILE[colorIdx % TILE.length];
  px(ctx, x, y, sz, sz, PAL.frame);
  px(ctx, x + 1, y + 1, sz - 2, sz - 2, t.base);
  px(ctx, x + 1, y + 1, sz - 2, Math.max(2, sz / 8 | 0), t.hi);
}

// ---- the board -----------------------------------------------------------
function drawBoard(ctx, s) {
  const g = gridGeom(s.cols, s.rows);
  px(ctx, g.ox - 4, g.oy - 4, g.bw + 8, g.bh + 8, PAL.frame);
  for (let r = 0; r < s.rows; r++) {
    for (let c = 0; c < s.cols; c++) {
      drawTile(ctx, g.ox + c * g.cell, g.oy + r * g.cell, g.cell, s.grid[r * s.cols + c]);
      if (!s.over && s.grid[r * s.cols + c] === s.target[r * s.cols + c]) {
        // a small pip marks a tile already matching the target
        px(ctx, g.ox + c * g.cell + g.cell / 2 - 2, g.oy + r * g.cell + g.cell / 2 - 2, 4, 4, PAL.frame);
      }
    }
  }
}

// ---- target preview ------------------------------------------------------
function drawTarget(ctx, s, cx, cy) {
  const mc = Math.min(9, Math.floor(54 / Math.max(s.cols, s.rows)));
  const w = mc * s.cols, h = mc * s.rows;
  const ox = cx - w / 2, oy = cy - h / 2;
  px(ctx, ox - 3, oy - 3, w + 6, h + 6, PAL.frame);
  for (let r = 0; r < s.rows; r++) {
    for (let c = 0; c < s.cols; c++) {
      px(ctx, ox + c * mc, oy + r * mc, mc, mc, TILE[s.target[r * s.cols + c] % TILE.length].base);
    }
  }
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  fillText(ctx, L(s.level.name).toUpperCase(), 116, 22, 16, PAL.text, 'left');
  fillText(ctx, t('moves'), 116, 46, 10, PAL.dim, 'left');
  fillText(ctx, s.moves + ' / ' + s.level.depth, 116, 62, 15,
    s.moves <= s.level.depth ? PAL.good : PAL.text, 'left');
  fillText(ctx, t('target'), 300, 12, 9, PAL.dim);
  drawTarget(ctx, s, 300, 42);
}

// ---- buttons + stars -----------------------------------------------------
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 44 ? 16 : 13, active ? PAL.ink : color);
}
function drawStars(ctx, cx, cy, n, size) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * (size + 6), on = i < n;
    px(ctx, x - size / 2, cy - size / 2, size, size, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy - size / 2 - 3, 6, 3, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy + size / 2, 6, 3, on ? PAL.star : PAL.panel);
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  const cell = 40, cols = 5, rows = 5;
  const ox = (360 - cell * cols) / 2, oy = 122;
  const shift = Math.floor(now / 600);
  for (let r = 0; r < rows; r++) {
    const off = ((r % 2 === 0 ? shift : -shift) % cols + cols) % cols;
    for (let c = 0; c < cols; c++) {
      const src = (c + off) % cols;
      const col = Math.min(2, Math.max(Math.abs(r - 2), Math.abs(src - 2)));
      drawTile(ctx, ox + c * cell, oy + r * cell, cell, [3, 4, 5][col]);
    }
  }
}
