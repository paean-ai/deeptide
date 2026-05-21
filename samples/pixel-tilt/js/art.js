// Pixel Tilt - all canvas drawing: cavern grid, crystals, controls, title.

const PAL = {
  bg0: '#16121f', bg1: '#221b30', frame: '#0b0912',
  floor: '#2b2540', floorAlt: '#322b4a', grid: '#1c1830',
  wall: '#5d5670', wallDk: '#363149', wallHi: '#7a7390',
  panel: '#2c2440', panelHi: '#3e3358', ink: '#0b0912',
  text: '#f3f1e6', dim: '#998fae', good: '#7bd88f', star: '#ffe27a',
};
// crystal / goal colours, indexed by crystal slot
const GEMS = [
  { base: '#e8554f', hi: '#ff8f88', dark: '#8c2b28' },
  { base: '#5fc06e', hi: '#9be88a', dark: '#2f7340' },
  { base: '#4a9be8', hi: '#8fc8ff', dark: '#235f96' },
  { base: '#f2c83a', hi: '#ffe88a', dark: '#9c7d1c' },
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
function boardGeom(n) {
  const cell = Math.min(46, Math.floor(282 / n));
  const bpx = cell * n;
  return { n, cell, bpx, ox: (360 - bpx) / 2 | 0, oy: 72 + ((288 - bpx) / 2 | 0) };
}
function cellXY(g, cell) {
  return { x: g.ox + (cell % g.n) * g.cell, y: g.oy + ((cell / g.n | 0)) * g.cell };
}
const DPAD = {
  U: { x: 248, y: 362, w: 44, h: 44 },
  D: { x: 248, y: 418, w: 44, h: 44 },
  L: { x: 200, y: 390, w: 44, h: 44 },
  R: { x: 296, y: 390, w: 44, h: 44 },
};
const UNDO_BTN = { x: 20, y: 366, w: 160, h: 40 };
const RESTART_BTN = { x: 20, y: 418, w: 160, h: 40 };

// ---- backdrop ------------------------------------------------------------
function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

function drawGem(ctx, cx, cy, rad, gi) {
  const g = GEMS[gi % GEMS.length];
  for (let dy = -rad; dy <= rad; dy++) {
    const w = (rad - Math.abs(dy)) * 2;
    px(ctx, cx - w, cy + dy, w * 2, 1, dy < 0 ? g.hi : g.base);
  }
  // facet shading
  for (let dy = 1; dy <= rad; dy++) {
    const w = (rad - dy) * 2;
    px(ctx, cx, cy + dy, w, 1, g.dark);
  }
  px(ctx, cx - rad + 2, cy - 1, 3, 3, '#ffffff');
}

// ---- the cavern board ----------------------------------------------------
// crystalPx: optional array of {x,y} pixel centres (for slide animation)
function drawBoard(ctx, s, crystalPx) {
  const g = boardGeom(s.n), n = s.n;
  px(ctx, g.ox - 5, g.oy - 5, g.bpx + 10, g.bpx + 10, PAL.frame);
  for (let i = 0; i < n * n; i++) {
    const p = cellXY(g, i);
    const isWall = s.wallSet.has(i);
    px(ctx, p.x, p.y, g.cell, g.cell, ((i + (i / n | 0)) % 2) ? PAL.floorAlt : PAL.floor);
    px(ctx, p.x, p.y, g.cell, 1, PAL.grid);
    px(ctx, p.x, p.y, 1, g.cell, PAL.grid);
    if (isWall) {
      px(ctx, p.x + 2, p.y + 2, g.cell - 4, g.cell - 4, PAL.wall);
      px(ctx, p.x + 2, p.y + 2, g.cell - 4, 3, PAL.wallHi);
      px(ctx, p.x + 2, p.y + g.cell - 5, g.cell - 4, 3, PAL.wallDk);
    }
  }
  // goal pads
  for (let i = 0; i < s.level.goals.length; i++) {
    const p = cellXY(g, s.level.goals[i]);
    const gm = GEMS[i % GEMS.length];
    const m = 5;
    for (let k = 0; k < 3; k++) {
      ctx.fillStyle = gm.base;
      ctx.fillRect(p.x + m + k, p.y + m + k, g.cell - 2 * m - 2 * k, 2);
      ctx.fillRect(p.x + m + k, p.y + g.cell - m - k - 2, g.cell - 2 * m - 2 * k, 2);
      ctx.fillRect(p.x + m + k, p.y + m + k, 2, g.cell - 2 * m - 2 * k);
      ctx.fillRect(p.x + g.cell - m - k - 2, p.y + m + k, 2, g.cell - 2 * m - 2 * k);
      break;
    }
    px(ctx, p.x + g.cell / 2 - 2, p.y + g.cell / 2 - 2, 4, 4, gm.dark);
  }
  // crystals
  const rad = Math.floor(g.cell * 0.32);
  for (let i = 0; i < s.pos.length; i++) {
    let cx, cy;
    if (crystalPx && crystalPx[i]) { cx = crystalPx[i].x; cy = crystalPx[i].y; }
    else { const p = cellXY(g, s.pos[i]); cx = p.x + g.cell / 2; cy = p.y + g.cell / 2; }
    const onGoal = s.pos[i] === s.level.goals[i];
    if (onGoal) {
      ctx.globalAlpha = 0.5;
      px(ctx, cx - rad - 3, cy - rad - 3, (rad + 3) * 2, (rad + 3) * 2, GEMS[i % GEMS.length].hi);
      ctx.globalAlpha = 1;
    }
    drawGem(ctx, cx | 0, cy | 0, rad, i);
  }
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  fillText(ctx, L(s.level.name).toUpperCase(), 180, 22, 17, PAL.text);
  fillText(ctx, t('moves'), 116, 46, 10, PAL.dim);
  fillText(ctx, String(s.moves), 116, 60, 16,
    s.moves <= s.level.par ? PAL.good : PAL.text);
  fillText(ctx, t('par'), 244, 46, 10, PAL.dim);
  fillText(ctx, String(s.level.par), 244, 60, 16, PAL.star);
}

// ---- buttons -------------------------------------------------------------
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 52 ? 16 : 13,
    active ? PAL.ink : color);
}
function drawArrowBtn(ctx, r, dir, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? PAL.panelHi : PAL.panel);
  const cx = r.x + r.w / 2, cy = r.y + r.h / 2, c = active ? PAL.star : PAL.dim;
  for (let k = 0; k < 7; k++) {
    if (dir === 'U') px(ctx, cx - k, cy + 5 - k, k * 2 + 1, 2, c);
    else if (dir === 'D') px(ctx, cx - k, cy - 5 + k, k * 2 + 1, 2, c);
    else if (dir === 'L') px(ctx, cx + 5 - k, cy - k, 2, k * 2 + 1, c);
    else px(ctx, cx - 5 + k, cy - k, 2, k * 2 + 1, c);
  }
}

// ---- stars ---------------------------------------------------------------
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
  const g = { n: 5, cell: 44, bpx: 220, ox: 70, oy: 120 };
  px(ctx, g.ox - 5, g.oy - 5, g.bpx + 10, g.bpx + 10, PAL.frame);
  for (let i = 0; i < 25; i++) {
    const p = cellXY(g, i);
    px(ctx, p.x, p.y, g.cell, g.cell, ((i + (i / 5 | 0)) % 2) ? PAL.floorAlt : PAL.floor);
    px(ctx, p.x, p.y, g.cell, 1, PAL.grid);
  }
  const slide = (Math.sin(now / 700) * 0.5 + 0.5);
  for (let i = 0; i < 4; i++) {
    const row = i + 1;
    const x = g.ox + (1 + slide * 2) * g.cell + g.cell / 2;
    const y = g.oy + row * g.cell + g.cell / 2;
    drawGem(ctx, x | 0, y | 0, 13, i);
  }
}
