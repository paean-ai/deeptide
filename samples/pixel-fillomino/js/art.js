// Pixel Fillomino - all canvas drawing: the grid, number pad, title.

const PAL = {
  bg0: '#181426', bg1: '#241d38', frame: '#0c0a16',
  panel: '#2b2444', panelHi: '#3d3358', ink: '#0c0a16',
  text: '#f3f1e6', dim: '#998fb0', good: '#7bd88f', star: '#ffe27a',
  cell: '#33304e', cellGiven: '#211d36', sel: '#4a4378',
  given: '#cfd8ff', filled: '#ffe27a', bad: '#ff6470', border: '#f3f1e6',
};

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// ---- layout --------------------------------------------------------------
function gridGeom(n) {
  const cell = Math.min(48, Math.floor(294 / n));
  const bpx = cell * n;
  return { cell, bpx, ox: (360 - bpx) / 2 | 0, oy: 66 + ((300 - bpx) / 2 | 0) };
}
function numBtn(i) { return { x: 8 + i * 43, y: 384, w: 41, h: 42 }; }   // 1..8
const ERASE_BTN = { x: 26, y: 432, w: 142, h: 36 };
const RESTART_BTN = { x: 192, y: 432, w: 142, h: 36 };

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

// ---- the board -----------------------------------------------------------
function drawBoard(ctx, s, sel) {
  const g = gridGeom(s.n), n = s.n;
  const bad = findViolations(n, s.grid);
  px(ctx, g.ox - 4, g.oy - 4, g.bpx + 8, g.bpx + 8, PAL.frame);
  for (let i = 0; i < n * n; i++) {
    const r = i / n | 0, c = i % n;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    const given = s.clues[i] !== 0;
    let bgc = given ? PAL.cellGiven : PAL.cell;
    if (i === sel && !given) bgc = PAL.sel;
    if (bad.has(i)) bgc = '#5a2230';
    px(ctx, x + 1, y + 1, g.cell - 2, g.cell - 2, bgc);
    const v = s.grid[i];
    if (v !== 0) {
      const col = bad.has(i) ? PAL.bad : (given ? PAL.given : PAL.filled);
      fillText(ctx, String(v), x + g.cell / 2, y + g.cell / 2 + 1, g.cell * 0.5 | 0, col);
    }
  }
  // region borders: thick line between adjacent cells of different numbers
  for (let i = 0; i < n * n; i++) {
    const r = i / n | 0, c = i % n;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell, v = s.grid[i];
    if (v === 0) continue;
    if (c + 1 < n && s.grid[i + 1] !== v) px(ctx, x + g.cell - 2, y, 3, g.cell, PAL.border);
    if (r + 1 < n && s.grid[i + n] !== v) px(ctx, x, y + g.cell - 2, g.cell, 3, PAL.border);
  }
  // outer frame edge
  px(ctx, g.ox - 1, g.oy - 1, g.bpx + 2, 2, PAL.border);
  px(ctx, g.ox - 1, g.oy + g.bpx - 1, g.bpx + 2, 2, PAL.border);
  px(ctx, g.ox - 1, g.oy - 1, 2, g.bpx + 2, PAL.border);
  px(ctx, g.ox + g.bpx - 1, g.oy - 1, 2, g.bpx + 2, PAL.border);
}

// ---- number pad ----------------------------------------------------------
function drawNumPad(ctx, s) {
  for (let v = 1; v <= MAX_NUM; v++) {
    const r = numBtn(v - 1);
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, PAL.panel);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, PAL.panelHi);
    fillText(ctx, String(v), r.x + r.w / 2, r.y + r.h / 2, 18, PAL.filled);
  }
}
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 44 ? 16 : 13, active ? PAL.ink : color);
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  fillText(ctx, L(s.level.name).toUpperCase(), 180, 24, 16, PAL.text);
  const left = s.n * s.n - filledCount(s);
  fillText(ctx, left + ' ' + t('left'), 180, 46, 11, left === 0 ? PAL.good : PAL.dim);
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
  const cell = 40, n = 5, ox = (360 - cell * n) / 2, oy = 120;
  const demo = [3, 3, 3, 4, 4, 2, 1, 5, 4, 4, 2, 5, 5, 5, 4, 6, 6, 5, 1, 2,
    6, 6, 6, 2, 2];
  for (let i = 0; i < 25; i++) {
    const r = i / 5 | 0, c = i % 5;
    const x = ox + c * cell, y = oy + r * cell;
    px(ctx, x + 1, y + 1, cell - 2, cell - 2, PAL.cell);
    fillText(ctx, String(demo[i]), x + cell / 2, y + cell / 2 + 1, 20, PAL.filled);
    if (c + 1 < 5 && demo[i + 1] !== demo[i]) px(ctx, x + cell - 2, y, 3, cell, PAL.border);
    if (r + 1 < 5 && demo[i + 5] !== demo[i]) px(ctx, x, y + cell - 2, cell, 3, PAL.border);
  }
}
