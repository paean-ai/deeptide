// Pixel-art rendering for Pixel Klotski. 360x480 world units.

const PALETTE = {
  bg:        '#1c1424',
  bgHi:      '#28203a',
  frame:     '#3a2a14',
  frameHi:   '#5c4220',
  frameLo:   '#1c130a',
  empty:     '#10121c',
  emptyHi:   '#1a1c30',
  target:    '#e85a3a',         // red 2x2 general
  targetHi:  '#ff8a6a',
  targetLo:  '#7a1e0c',
  block2x1:  '#5fc06e',
  block2x1Hi:'#86df9d',
  block2x1Lo:'#2d6038',
  block1x2:  '#5fc0ff',
  block1x2Hi:'#82c0ff',
  block1x2Lo:'#205a8a',
  block1x1:  '#f4d27b',
  block1x1Hi:'#fff0c8',
  block1x1Lo:'#9a8048',
  blockOther:'#bda6ff',
  blockOtherHi:'#e3d3ff',
  blockOtherLo:'#5a4a80',
  select:    '#ffe04a',
  goal:      '#5fc06e',
  hud:       '#06031a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  star:      '#f8d34a',
  starOff:   '#3a4274',
  win:       '#5fc06e',
};

// Geometry: board is centred under the HUD. Cell size adapts to the
// (BOARD_W, BOARD_H) constants so the same renderer also works for
// future variants. 4x5 -> cell 56.
function gridGeometry() {
  const cell = Math.min(60, Math.floor(300 / BOARD_W), Math.floor(320 / BOARD_H));
  const bw = cell * BOARD_W, bh = cell * BOARD_H;
  const ox = ((360 - bw) / 2) | 0;
  const oy = 60;
  return { cell, bw, bh, ox, oy };
}
function cellRect(g, cx, cy) {
  return { x: g.ox + cx * g.cell, y: g.oy + cy * g.cell, w: g.cell, h: g.cell };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgHi;
  for (let i = 0; i < 28; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s) {
  const g = gridGeometry();
  // Outer frame.
  ctx.fillStyle = PALETTE.frameLo;
  ctx.fillRect(g.ox - 7, g.oy - 7, g.bw + 14, g.bh + 14);
  ctx.fillStyle = PALETTE.frame;
  ctx.fillRect(g.ox - 6, g.oy - 6, g.bw + 12, g.bh + 12);
  ctx.fillStyle = PALETTE.frameHi;
  ctx.fillRect(g.ox - 6, g.oy - 6, g.bw + 12, 3);
  // Empty cells.
  for (let r = 0; r < BOARD_H; r++) for (let c = 0; c < BOARD_W; c++) {
    const rect = cellRect(g, c, r);
    ctx.fillStyle = PALETTE.empty;
    ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
    ctx.fillStyle = PALETTE.emptyHi;
    ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, 1);
  }
  // Goal slot — a soft green outline around the target's expected position.
  const goalR = cellRect(g, s.lv.goal.x, s.lv.goal.y);
  ctx.strokeStyle = PALETTE.goal;
  ctx.lineWidth = 2;
  ctx.strokeRect(goalR.x + 1, goalR.y + 1, g.cell * 2 - 2, g.cell * 2 - 2);
  // Pieces.
  for (let i = 0; i < s.pieces.length; i++) drawPiece(ctx, g, s.pieces[i], i === s.selected, i === 0);
}

function drawPiece(ctx, g, p, selected, isTarget) {
  const x = g.ox + p.x * g.cell;
  const y = g.oy + p.y * g.cell;
  const w = p.w * g.cell, h = p.h * g.cell;
  let body, hi, lo;
  if (isTarget) { body = PALETTE.target; hi = PALETTE.targetHi; lo = PALETTE.targetLo; }
  else if (p.w === 2 && p.h === 1) { body = PALETTE.block2x1; hi = PALETTE.block2x1Hi; lo = PALETTE.block2x1Lo; }
  else if (p.w === 1 && p.h === 2) { body = PALETTE.block1x2; hi = PALETTE.block1x2Hi; lo = PALETTE.block1x2Lo; }
  else if (p.w === 1 && p.h === 1) { body = PALETTE.block1x1; hi = PALETTE.block1x1Hi; lo = PALETTE.block1x1Lo; }
  else { body = PALETTE.blockOther; hi = PALETTE.blockOtherHi; lo = PALETTE.blockOtherLo; }
  const m = 3;
  ctx.fillStyle = PALETTE.frameLo;
  ctx.fillRect(x + m - 1, y + m - 1, w - m * 2 + 2, h - m * 2 + 2);
  ctx.fillStyle = body;
  ctx.fillRect(x + m, y + m, w - m * 2, h - m * 2);
  ctx.fillStyle = hi;
  ctx.fillRect(x + m, y + m, w - m * 2, 3);
  ctx.fillRect(x + m, y + m, 3, h - m * 2);
  ctx.fillStyle = lo;
  ctx.fillRect(x + m, y + h - m - 3, w - m * 2, 3);
  ctx.fillRect(x + w - m - 3, y + m, 3, h - m * 2);
  if (isTarget) {
    // Centre kanji-ish glyph for "general" — a pixel diamond.
    const cx = x + w / 2, cy = y + h / 2;
    ctx.fillStyle = hi;
    for (let i = 0; i < 6; i++) {
      ctx.fillRect((cx - 5 + i) | 0, (cy - 5 + i) | 0, 1, 1);
      ctx.fillRect((cx + 4 - i) | 0, (cy - 5 + i) | 0, 1, 1);
    }
  }
  if (selected) {
    ctx.strokeStyle = PALETTE.select;
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 2, y + 2, w - 4, h - 4);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves + ' / ' + t(lang, 'par') + ' ' + s.lv.par, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'best') + ' ' + (best || '—'), VW - 6, 16);
}

function drawStars(ctx, x, y, n, w = 14) {
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < n ? PALETTE.star : PALETTE.starOff;
    drawStar(ctx, x + i * (w + 4) + w / 2, y, w / 2);
  }
}
function drawStar(ctx, cx, cy, r) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const a = (Math.PI / 5) * i - Math.PI / 2;
    const rr = i % 2 === 0 ? r : r * 0.45;
    ctx.lineTo(cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
  }
  ctx.closePath();
  ctx.fill();
}
