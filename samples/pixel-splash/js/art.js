// Pixel Splash - all canvas drawing: the paint grid, swatches, HUD, title.

const PAL = {
  bg0: '#191526', bg1: '#241d36', frame: '#0d0a14',
  stone: '#6b6478', stoneDk: '#3f3a4d', stoneHi: '#8b849a',
  splash: '#fdf6e3',
  panel: '#2c2440', panelHi: '#3e3358', ink: '#0d0a14',
  text: '#f3f1e6', dim: '#998fad', good: '#7bd88f', warn: '#f2a83e', bad: '#ff6470',
  star: '#ffe27a',
};
const PAINTS = ['#e8554f', '#f2a23a', '#f2d34a', '#5fc06e', '#4a9be8', '#9a6cd8'];
const PAINT_HI = ['#ff8079', '#ffc66a', '#fff08a', '#8fe09a', '#7cc1ff', '#c39bf2'];

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// ---- layout --------------------------------------------------------------
function boardGeom(n) {
  const cell = Math.min(40, Math.floor(290 / n));
  const bpx = cell * n;
  const ox = (360 - bpx) / 2 | 0;
  const oy = 76 + ((300 - bpx) / 2 | 0);
  return { cell, bpx, ox, oy };
}
function swatchRects(colors) {
  const w = Math.min(50, Math.floor(312 / colors));
  const total = colors * w + (colors - 1) * 8;
  const x0 = (360 - total) / 2 | 0;
  const rects = [];
  for (let i = 0; i < colors; i++) rects.push({ x: x0 + i * (w + 8), y: 384, w, h: 46 });
  return rects;
}
const UNDO_BTN = { x: 46, y: 438, w: 130, h: 34 };
const RESTART_BTN = { x: 184, y: 438, w: 130, h: 34 };

// ---- backdrop ------------------------------------------------------------
function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

// ---- the paint grid ------------------------------------------------------
function drawCanvasBoard(ctx, s) {
  const L = s.level, n = L.n, g = boardGeom(n);
  px(ctx, g.ox - 5, g.oy - 5, g.bpx + 10, g.bpx + 10, PAL.frame);
  for (let i = 0; i < n * n; i++) {
    const r = (i / n) | 0, c = i % n;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    if (s.board[i] === STONE) {
      px(ctx, x, y, g.cell, g.cell, PAL.stoneDk);
      px(ctx, x + 2, y + 2, g.cell - 4, g.cell - 4, PAL.stone);
      px(ctx, x + 3, y + 3, g.cell - 6, 2, PAL.stoneHi);
      px(ctx, x + (g.cell * 0.55 | 0), y + (g.cell * 0.3 | 0), 2, g.cell * 0.4 | 0, PAL.stoneDk);
    } else {
      px(ctx, x, y, g.cell, g.cell, PAINTS[s.board[i]]);
      px(ctx, x, y, g.cell, 2, PAINT_HI[s.board[i]]);
    }
    px(ctx, x, y + g.cell - 1, g.cell, 1, PAL.frame);
    px(ctx, x + g.cell - 1, y, 1, g.cell, PAL.frame);
  }
  // outline the current splash so its reach is clear
  const region = originRegion(s.board, n);
  for (let i = 0; i < n * n; i++) {
    if (!region[i]) continue;
    const r = (i / n) | 0, c = i % n;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    if (r === 0 || !region[i - n]) px(ctx, x, y, g.cell, 3, PAL.splash);
    if (r === n - 1 || !region[i + n]) px(ctx, x, y + g.cell - 3, g.cell, 3, PAL.splash);
    if (c === 0 || !region[i - 1]) px(ctx, x, y, 3, g.cell, PAL.splash);
    if (c === n - 1 || !region[i + 1]) px(ctx, x + g.cell - 3, y, 3, g.cell, PAL.splash);
  }
  // origin pip
  const hc = g.cell / 2 | 0;
  px(ctx, g.ox + hc - 3, g.oy + hc - 3, 6, 6, PAL.splash);
}

// ---- swatches ------------------------------------------------------------
function drawSwatches(ctx, s) {
  const rects = swatchRects(s.level.colors);
  const cur = s.board[0];
  for (let c = 0; c < rects.length; c++) {
    const r = rects[c], active = c !== cur && !s.over;
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    const inset = active ? 3 : 6;
    px(ctx, r.x + inset, r.y + inset, r.w - inset * 2, r.h - inset * 2, PAINTS[c]);
    if (active) px(ctx, r.x + inset, r.y + inset, r.w - inset * 2, 3, PAINT_HI[c]);
    if (c === cur) {
      // mark the splash's own colour
      px(ctx, r.x + r.w / 2 - 5, r.y + r.h / 2 - 2, 10, 4, PAL.ink);
    }
  }
}

function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 40 ? 15 : 12,
    active ? PAL.ink : color);
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  const L = s.level;
  fillText(ctx, L.name[lang === 'zh' ? 1 : 0].toUpperCase(), 180, 22, 16, PAL.text);
  const left = L.budget - s.moves;
  const lowC = left <= 2 ? PAL.bad : (left <= 4 ? PAL.warn : PAL.good);
  fillText(ctx, t('moves'), 96, 46, 10, PAL.dim);
  fillText(ctx, s.moves + ' / ' + L.budget, 96, 60, 15, lowC);
  fillText(ctx, t('par'), 264, 46, 10, PAL.dim);
  fillText(ctx, String(L.par), 264, 60, 15, PAL.star);
  // budget pips
  for (let i = 0; i < L.budget; i++) {
    const x = 22 + i * (Math.min(11, 316 / L.budget));
    px(ctx, x, 68, Math.min(8, 316 / L.budget - 2), 4, i < s.moves ? PAL.panel : lowC);
  }
}

// ---- stars ---------------------------------------------------------------
function drawStars(ctx, cx, cy, n, size) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * (size + 6);
    const on = i < n;
    px(ctx, x - size / 2, cy - size / 2, size, size, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy - size / 2 - 3, 6, 3, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy + size / 2, 6, 3, on ? PAL.star : PAL.panel);
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  // a canvas with paint blobs spreading
  const cx = 180, cy = 188, cell = 30;
  px(ctx, cx - cell * 2.5 - 5, cy - cell * 2.5 - 5, cell * 5 + 10, cell * 5 + 10, PAL.frame);
  for (let r = 0; r < 5; r++) for (let c = 0; c < 5; c++) {
    const ring = Math.max(Math.abs(r - 2), Math.abs(c - 2));
    const phase = (now / 420) % 6;
    const lit = phase > ring;
    const col = PAINTS[(r + c) % 6];
    const x = cx - cell * 2.5 + c * cell, y = cy - cell * 2.5 + r * cell;
    px(ctx, x, y, cell, cell, lit ? col : PAL.panel);
    if (lit) px(ctx, x, y, cell, 3, PAINT_HI[(r + c) % 6]);
    px(ctx, x, y + cell - 1, cell, 1, PAL.frame);
    px(ctx, x + cell - 1, y, 1, cell, PAL.frame);
  }
  px(ctx, cx - 4, cy - 4, 8, 8, PAL.splash);
}
