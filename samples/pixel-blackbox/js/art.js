// Pixel-art rendering for Pixel Black Box. 360x480 world units.

const PALETTE = {
  bg:        '#0d0918',
  card:      '#1a1230',
  cardEdge:  '#070315',
  cell:      '#1f1838',
  cellHi:    '#2d2452',
  cellSus:   '#3a2c5a',
  border:    '#070315',
  hud:       '#070315',
  hudText:   '#f8f5e8',
  hudDim:    '#a8a0c4',
  edge:      '#3a2c5a',
  edgeText:  '#c8c0e0',
  edgeFired: '#1f1838',
  resultHit: '#ff5a5a',
  resultRefl:'#5fc0ff',
  pass:      '#f4d27b',
  mark:      '#ffd34a',
  markBg:    '#3a2c00',
  atomReal:  '#bda6ff',
  atomRealHi:'#e3d3ff',
  atomMiss:  '#9affe8',
  border2:   '#0a0517',
  bright:    '#ffd34a',
  win:       '#5fc06e',
  bad:       '#ff5a5a',
};

// Geometry: grid centred + 1-row edge strip on all sides for probe buttons.
// Total layout = (n+2) x (n+2) cells. Cell size adapts to n.
function gridGeometry(n) {
  const cell = n === 6 ? 36 : n === 7 ? 32 : n === 8 ? 28 : 25;  // 6/7/8/9
  const total = (n + 2) * cell;
  const ox = ((VW - total) / 2) | 0;
  const oy = 70;
  return { cell, total, ox, oy };
}

function gridCellRect(n, x, y) {
  const g = gridGeometry(n);
  return { x: g.ox + (x + 1) * g.cell, y: g.oy + (y + 1) * g.cell, w: g.cell, h: g.cell };
}

// Each edge index maps to one of 4n outer cells (top row, right col, bottom row, left col).
function edgeRect(n, idx) {
  const g = gridGeometry(n);
  const e = idx % (4 * n);
  if (e < n)       return { x: g.ox + (e + 1) * g.cell,         y: g.oy,                       w: g.cell, h: g.cell };
  if (e < 2 * n)   return { x: g.ox + (n + 1) * g.cell,         y: g.oy + (e - n + 1) * g.cell, w: g.cell, h: g.cell };
  if (e < 3 * n)   return { x: g.ox + (3 * n - 1 - e + 1) * g.cell, y: g.oy + (n + 1) * g.cell, w: g.cell, h: g.cell };
                   return { x: g.ox,                              y: g.oy + (4 * n - 1 - e + 1) * g.cell, w: g.cell, h: g.cell };
}

function drawBackdrop(ctx, n) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  const g = gridGeometry(n);
  // Frame card behind the playfield.
  ctx.fillStyle = PALETTE.cardEdge;
  ctx.fillRect(g.ox - 4, g.oy - 4, g.total + 8, g.total + 8);
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(g.ox, g.oy, g.total, g.total);
}

function drawBoard(ctx, s) {
  drawBackdrop(ctx, s.n);
  // Interior cells (the black box).
  for (let y = 0; y < s.n; y++) for (let x = 0; x < s.n; x++) {
    const r = gridCellRect(s.n, x, y);
    ctx.fillStyle = (x + y) % 2 === 0 ? PALETTE.cell : PALETTE.cellHi;
    ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    if (s.marks[y][x]) {
      ctx.fillStyle = PALETTE.markBg;
      ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
      // X mark
      ctx.fillStyle = PALETTE.mark;
      ctx.fillRect(r.x + r.w / 2 - 5, r.y + r.h / 2 - 5, 10, 2);
      ctx.fillRect(r.x + r.w / 2 - 5, r.y + r.h / 2 + 3, 10, 2);
      ctx.fillRect(r.x + r.w / 2 - 5, r.y + r.h / 2 - 1, 2, 2);
      ctx.fillRect(r.x + r.w / 2 + 3, r.y + r.h / 2 - 1, 2, 2);
    }
    if (s.revealed) {
      const isAtom = hasAtom(s.atoms, x, y);
      const isMark = s.marks[y][x];
      if (isAtom && isMark)        drawAtom(ctx, r, PALETTE.win);
      else if (isAtom && !isMark)  drawAtom(ctx, r, PALETTE.atomReal);
      else if (!isAtom && isMark)  drawAtomMiss(ctx, r);
    }
  }
  // Edge probe buttons.
  for (let i = 0; i < 4 * s.n; i++) {
    const r = edgeRect(s.n, i);
    const fired = s.fired[i];
    ctx.fillStyle = fired ? PALETTE.edgeFired : PALETTE.edge;
    ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    if (fired) {
      let txt = '?';
      let col = PALETTE.edgeText;
      if (fired.kind === 'hit')      { txt = 'H'; col = PALETTE.resultHit; }
      else if (fired.kind === 'reflect') { txt = 'R'; col = PALETTE.resultRefl; }
      else if (fired.kind === 'pass' || fired.kind === 'passOut') { txt = s.edgeLabel[i] || '?'; col = PALETTE.pass; }
      ctx.fillStyle = col;
      ctx.font = 'bold 14px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(txt, r.x + r.w / 2, r.y + r.h / 2 + 1);
    } else {
      // Indicate it's a probe button.
      ctx.fillStyle = PALETTE.edgeText;
      ctx.font = '10px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('·', r.x + r.w / 2, r.y + r.h / 2);
    }
  }
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}
function drawAtom(ctx, r, col) {
  const cx = r.x + r.w / 2, cy = r.y + r.h / 2;
  ctx.fillStyle = '#0a0517';
  fillDisk(ctx, cx, cy + 1, r.w * 0.34);
  ctx.fillStyle = col;
  fillDisk(ctx, cx, cy, r.w * 0.32);
  ctx.fillStyle = PALETTE.atomRealHi;
  fillDisk(ctx, cx - 2, cy - 2, r.w * 0.18);
}
function drawAtomMiss(ctx, r) {
  // Wrong guess: red ring through the mark.
  const cx = r.x + r.w / 2, cy = r.y + r.h / 2;
  ctx.strokeStyle = PALETTE.bad;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.arc(cx, cy, r.w * 0.32, 0, Math.PI * 2);
  ctx.stroke();
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
  ctx.fillText(t(lang, 'atoms') + ' ' + s.lv.atoms + '  ' + t(lang, 'marks') + ' ' + markCount(s), VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'probes') + ' ' + probeCount(s), VW - 6, 16);
}
