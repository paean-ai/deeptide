// Pixel-art rendering for Pixel Dice. 360x480 world units.

const PALETTE = {
  bg:       '#1a1726',
  panel:    '#272336',
  panelHi:  '#39334c',
  cell:     '#322c44',
  cellEdge: '#423a58',
  reach:    '#3c4f6e',
  die:      '#f3ecda',
  dieHi:    '#ffffff',
  dieLo:    '#b8ad8e',
  pip:      '#2a2436',
  seal:     '#4a4360',
  sealRing: '#8a7fb0',
  sealSet:  '#5fc06e',
  sealPip:  '#cdbfe8',
  hud:      '#0e0c16',
  hudText:  '#f3f1e6',
  hudDim:   '#8a84a0',
  accent:   '#f4c44a',
  good:     '#5fc06e',
};

function gridGeometry(n) {
  const cell = Math.min(46, (296 / n) | 0);
  const span = cell * n;
  return { cell, span, ox: ((VW - span) / 2) | 0, oy: 70 };
}

// Pip layout for a die face value 1..6.
function pipSpots(v) {
  const L = -1, M = 0, R = 1;
  const map = {
    1: [[M, M]],
    2: [[L, L], [R, R]],
    3: [[L, L], [M, M], [R, R]],
    4: [[L, L], [R, L], [L, R], [R, R]],
    5: [[L, L], [R, L], [M, M], [L, R], [R, R]],
    6: [[L, L], [R, L], [L, M], [R, M], [L, R], [R, R]],
  };
  return map[v] || [];
}
function drawPips(ctx, cx, cy, half, val, color) {
  ctx.fillStyle = color;
  const r = Math.max(1.5, half * 0.2);
  for (const [px, py] of pipSpots(val)) {
    ctx.beginPath();
    ctx.arc(cx + px * half * 0.62, cy + py * half * 0.62, r, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawDie(ctx, x, y, cell, ori) {
  // Body.
  ctx.fillStyle = PALETTE.dieLo;
  ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
  ctx.fillStyle = PALETTE.die;
  ctx.fillRect(x + 3, y + 3, cell - 6, cell - 6);
  ctx.fillStyle = PALETTE.dieHi;
  ctx.fillRect(x + 3, y + 3, cell - 6, 3);
  // Top-face pips, large and central.
  drawPips(ctx, x + cell / 2, y + cell / 2, cell * 0.34, ori.t, PALETTE.pip);
  // Side faces as small digits at the four edges.
  ctx.fillStyle = '#9890ac';
  ctx.font = 'bold 8px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(ori.n), x + cell / 2, y + 7);
  ctx.fillText(String(ori.s), x + cell / 2, y + cell - 7);
  ctx.fillText(String(ori.e), x + cell - 7, y + cell / 2);
  ctx.fillText(String(ori.w), x + 7, y + cell / 2);
}

function drawBoard(ctx, s) {
  const p = s.puzzle, g = gridGeometry(p.n);
  ctx.fillStyle = PALETTE.panel;
  ctx.fillRect(g.ox - 8, g.oy - 8, g.span + 16, g.span + 16);
  ctx.fillStyle = PALETTE.panelHi;
  ctx.fillRect(g.ox - 8, g.oy - 8, g.span + 16, 3);
  // Reachable cells (adjacent to the die) get a subtle tint.
  const reach = {};
  if (!s.over) for (const [dir] of DIRS) {
    const r = adjacentRoll(s, dir);
    if (r) reach[r.cell] = true;
  }
  for (let i = 0; i < p.n * p.n; i++) {
    const x = g.ox + (i % p.n) * g.cell, y = g.oy + ((i / p.n) | 0) * g.cell;
    ctx.fillStyle = reach[i] ? PALETTE.reach : PALETTE.cell;
    ctx.fillRect(x + 1, y + 1, g.cell - 2, g.cell - 2);
    ctx.fillStyle = PALETTE.cellEdge;
    ctx.fillRect(x + 1, y + 1, g.cell - 2, 1);
    // Seal.
    if (p.goals[i] !== undefined) {
      const set = s.stamped[i];
      ctx.strokeStyle = set ? PALETTE.sealSet : PALETTE.sealRing;
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 5, y + 5, g.cell - 10, g.cell - 10);
      drawPips(ctx, x + g.cell / 2, y + g.cell / 2, g.cell * 0.3, p.goals[i],
               set ? PALETTE.sealSet : PALETTE.sealPip);
    }
  }
  // The die.
  drawDie(ctx, g.ox + (s.cell % p.n) * g.cell, g.oy + ((s.cell / p.n) | 0) * g.cell, g.cell, s.ori);
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.puzzle.levelIndex + 1) + ' ' + s.puzzle.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  let set = 0, total = 0;
  for (const c in s.puzzle.goals) { total++; if (s.stamped[c]) set++; }
  ctx.fillText(set + '/' + total, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves + ' / ' + t(lang, 'par') + ' ' + s.puzzle.par, VW - 8, 16);
}

function drawTitleArt(ctx, cx, cy) {
  drawDie(ctx, cx - 26, cy - 26, 52, { t: 5, n: 2, s: 5, e: 3, w: 4 });
  ctx.strokeStyle = PALETTE.sealRing;
  ctx.lineWidth = 2;
  for (const [dx, dy, v] of [[-72, 10, 3], [70, -20, 6], [56, 44, 1]]) {
    ctx.strokeRect(cx + dx - 16, cy + dy - 16, 32, 32);
    drawPips(ctx, cx + dx, cy + dy, 13, v, PALETTE.sealPip);
  }
}
