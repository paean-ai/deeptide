// Pixel-art rendering for Pixel Magnets. 360x480 world units.

const PALETTE = {
  bg:        '#1d2240',
  cell:      '#3a4274',
  cellSel:   '#7d8ed8',
  domEdge:   '#f8f5e8',
  pos:       '#e8554f',
  posDark:   '#a8373a',
  neg:       '#4a9be8',
  negDark:   '#1f5494',
  neutral:   '#5a6188',
  border:    '#07091a',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  conflict:  '#f7e69a',
  ok:        '#54c47c',
};

function gridGeometry(W, H) {
  // Reserve room for the count labels around the board (left + top + right + bottom).
  const cell = (W <= 4 && H <= 4) ? 44 : (W <= 6 && H <= 4) ? 40 : (W <= 4 && H <= 6) ? 40 : 34;
  const totalW = W * cell;
  const totalH = H * cell;
  const ox = ((360 - totalW) / 2) | 0;
  const oy = ((350 - totalH) / 2) + 36 | 0;
  return { cell, totalW, totalH, ox, oy };
}

function drawScene(ctx, p, marks, conflicts, selected) {
  const g = gridGeometry(p.W, p.H);
  drawCounts(ctx, p, g);
  // Cells.
  for (let y = 0; y < p.H; y++) for (let x = 0; x < p.W; x++) {
    const i = y * p.W + x;
    const px = g.ox + x * g.cell, py = g.oy + y * g.cell;
    let fill = PALETTE.cell;
    if (selected && selected[0] === x && selected[1] === y) fill = PALETTE.cellSel;
    ctx.fillStyle = fill;
    ctx.fillRect(px, py, g.cell, g.cell);
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(247, 230, 154, 0.45)';
      ctx.fillRect(px, py, g.cell, g.cell);
    }
    drawCellMark(ctx, marks[i], px, py, g.cell);
    // Cell border (thin).
    ctx.strokeStyle = PALETTE.border;
    ctx.lineWidth = 1;
    ctx.strokeRect(px + 0.5, py + 0.5, g.cell - 1, g.cell - 1);
  }
  // Domino borders (thick).
  const cellDom = new Array(p.W * p.H);
  p.dominoes.forEach((d, id) => d.forEach(c => cellDom[c] = id));
  ctx.strokeStyle = PALETTE.domEdge;
  ctx.lineWidth = 2;
  for (let y = 0; y < p.H; y++) for (let x = 0; x < p.W; x++) {
    const i = y * p.W + x;
    const px = g.ox + x * g.cell, py = g.oy + y * g.cell;
    if (x === p.W - 1 || cellDom[y * p.W + x + 1] !== cellDom[i]) {
      ctx.beginPath(); ctx.moveTo(px + g.cell + 0.5, py); ctx.lineTo(px + g.cell + 0.5, py + g.cell); ctx.stroke();
    }
    if (y === p.H - 1 || cellDom[(y + 1) * p.W + x] !== cellDom[i]) {
      ctx.beginPath(); ctx.moveTo(px, py + g.cell + 0.5); ctx.lineTo(px + g.cell, py + g.cell + 0.5); ctx.stroke();
    }
    if (y === 0) { ctx.beginPath(); ctx.moveTo(px, py + 0.5); ctx.lineTo(px + g.cell, py + 0.5); ctx.stroke(); }
    if (x === 0) { ctx.beginPath(); ctx.moveTo(px + 0.5, py); ctx.lineTo(px + 0.5, py + g.cell); ctx.stroke(); }
  }
}

function drawCellMark(ctx, v, px, py, cell) {
  if (v === 0) return;            // UNKNOWN
  if (v === 1) {                  // POS
    ctx.fillStyle = PALETTE.posDark;
    ctx.fillRect(px + 2, py + 2, cell - 4, cell - 4);
    ctx.fillStyle = PALETTE.pos;
    ctx.fillRect(px + 3, py + 3, cell - 6, cell - 6);
    ctx.fillStyle = '#fff';
    const t = (cell - 12) | 0, c = (cell / 2) | 0;
    ctx.fillRect(px + c - 1, py + c - t / 2 - 1, 3, t + 3);
    ctx.fillRect(px + c - t / 2 - 1, py + c - 1, t + 3, 3);
  } else if (v === 2) {           // NEG
    ctx.fillStyle = PALETTE.negDark;
    ctx.fillRect(px + 2, py + 2, cell - 4, cell - 4);
    ctx.fillStyle = PALETTE.neg;
    ctx.fillRect(px + 3, py + 3, cell - 6, cell - 6);
    ctx.fillStyle = '#fff';
    const t = (cell - 12) | 0, c = (cell / 2) | 0;
    ctx.fillRect(px + c - t / 2 - 1, py + c - 1, t + 3, 3);
  } else if (v === 3) {           // NEUTRAL (X)
    ctx.strokeStyle = PALETTE.neutral;
    ctx.lineWidth = 2;
    const m = 6;
    ctx.beginPath();
    ctx.moveTo(px + m, py + m); ctx.lineTo(px + cell - m, py + cell - m);
    ctx.moveTo(px + cell - m, py + m); ctx.lineTo(px + m, py + cell - m);
    ctx.stroke();
  }
}

function drawCounts(ctx, p, g) {
  // Left: rowP (positive counts). Right: rowN.
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 11px monospace';
  ctx.textBaseline = 'middle';
  for (let y = 0; y < p.H; y++) {
    const py = g.oy + y * g.cell + g.cell / 2;
    ctx.textAlign = 'right';
    ctx.fillStyle = PALETTE.pos;
    ctx.fillText(String(p.rc.rowP[y]), g.ox - 4, py);
    ctx.textAlign = 'left';
    ctx.fillStyle = PALETTE.neg;
    ctx.fillText(String(p.rc.rowN[y]), g.ox + g.totalW + 4, py);
  }
  // Top: colP. Bottom: colN.
  ctx.textAlign = 'center';
  for (let x = 0; x < p.W; x++) {
    const px = g.ox + x * g.cell + g.cell / 2;
    ctx.fillStyle = PALETTE.pos;
    ctx.fillText(String(p.rc.colP[x]), px, g.oy - 8);
    ctx.fillStyle = PALETTE.neg;
    ctx.fillText(String(p.rc.colN[x]), px, g.oy + g.totalH + 10);
  }
}

function drawHud(ctx, lang, p, marks, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (p.levelIndex + 1) + ' ' + p.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 180, 16);
  ctx.textAlign = 'right';
  const filled = marks.filter(m => m !== UNKNOWN).length;
  ctx.fillText(filled + '/' + (p.W * p.H), 352, 16);
}
