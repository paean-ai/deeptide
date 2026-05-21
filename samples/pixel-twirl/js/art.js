// Pixel Twirl - all canvas drawing: the board, target preview, title.

const PAL = {
  bg0: '#16142a', bg1: '#241f3e', frame: '#0c0a18',
  panel: '#2b2444', panelHi: '#3d335c', ink: '#0c0a18',
  text: '#f3f1e6', dim: '#998fb0', good: '#7bd88f', star: '#ffe27a',
};
const TILE = [
  { base: '#e8554f', hi: '#ff8f88' },
  { base: '#f2a83e', hi: '#ffca78' },
  { base: '#5fc06e', hi: '#9be88a' },
  { base: '#4a9be8', hi: '#8fc8ff' },
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
  const cell = Math.min(52, Math.floor(280 / Math.max(cols, rows)));
  const bw = cell * cols, bh = cell * rows;
  return { cell, bw, bh, ox: (360 - bw) / 2 | 0, oy: 88 + ((280 - bh) / 2 | 0) };
}
const UNDO_BTN = { x: 14, y: 400, w: 104, h: 40 };
const SPIN_BTN = { x: 128, y: 400, w: 104, h: 40 };
const RESTART_BTN = { x: 242, y: 400, w: 104, h: 40 };

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

function drawTile(ctx, x, y, sz, colorIdx) {
  const tl = TILE[colorIdx % TILE.length];
  px(ctx, x, y, sz, sz, PAL.frame);
  px(ctx, x + 1, y + 1, sz - 2, sz - 2, tl.base);
  px(ctx, x + 1, y + 1, sz - 2, Math.max(2, sz / 8 | 0), tl.hi);
}

// ---- the board -----------------------------------------------------------
function drawBoard(ctx, s) {
  const g = gridGeom(s.cols, s.rows);
  px(ctx, g.ox - 4, g.oy - 4, g.bw + 8, g.bh + 8, PAL.frame);
  for (let r = 0; r < s.rows; r++) {
    for (let c = 0; c < s.cols; c++) {
      drawTile(ctx, g.ox + c * g.cell, g.oy + r * g.cell, g.cell, s.grid[r * s.cols + c]);
      if (!s.over && s.grid[r * s.cols + c] === s.target[r * s.cols + c]) {
        px(ctx, g.ox + c * g.cell + g.cell / 2 - 2, g.oy + r * g.cell + g.cell / 2 - 2, 4, 4, PAL.frame);
      }
    }
  }
  // junction dots so the rotatable points read clearly
  for (let r = 1; r < s.rows; r++) {
    for (let c = 1; c < s.cols; c++) {
      px(ctx, g.ox + c * g.cell - 2, g.oy + r * g.cell - 2, 4, 4, PAL.ink);
    }
  }
}

function drawTarget(ctx, s, cx, cy) {
  const mc = Math.min(10, Math.floor(56 / Math.max(s.cols, s.rows)));
  const w = mc * s.cols, h = mc * s.rows;
  const ox = cx - w / 2, oy = cy - h / 2;
  px(ctx, ox - 3, oy - 3, w + 6, h + 6, PAL.frame);
  for (let r = 0; r < s.rows; r++) {
    for (let c = 0; c < s.cols; c++) {
      px(ctx, ox + c * mc, oy + r * mc, mc, mc, TILE[s.target[r * s.cols + c] % TILE.length].base);
    }
  }
}

function drawHud(ctx, s) {
  fillText(ctx, L(s.level.name).toUpperCase(), 116, 24, 16, PAL.text, 'left');
  fillText(ctx, t('moves'), 116, 48, 10, PAL.dim, 'left');
  fillText(ctx, s.moves + ' / ' + s.level.depth, 116, 64, 15,
    s.moves <= s.level.depth ? PAL.good : PAL.text, 'left');
  fillText(ctx, t('target'), 300, 14, 9, PAL.dim);
  drawTarget(ctx, s, 300, 46);
}

// ---- buttons + stars -----------------------------------------------------
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 44 ? 16 : 13, active ? PAL.ink : color);
}
function drawSpinBtn(ctx, dir) {
  const r = SPIN_BTN;
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, PAL.panelHi);
  fillText(ctx, (dir > 0 ? '↻ ' : '↺ ') + t('spin'), r.x + r.w / 2, r.y + r.h / 2, 13, PAL.star);
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
  const cell = 44, cols = 5, rows = 5;
  const ox = (360 - cell * cols) / 2, oy = 122;
  const spin = (now / 900) % (Math.PI * 2);
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const col = Math.min(3, Math.max(Math.abs(r - 2), Math.abs(c - 2)));
      drawTile(ctx, ox + c * cell, oy + r * cell, cell, col);
    }
  }
  // a spinning highlight on the centre block
  ctx.save();
  ctx.translate(ox + cell * 2.5, oy + cell * 2.5);
  ctx.rotate(spin);
  ctx.globalAlpha = 0.6;
  px(ctx, -cell + 4, -cell + 4, cell * 2 - 8, cell * 2 - 8, PAL.star);
  ctx.globalAlpha = 1;
  ctx.restore();
}
