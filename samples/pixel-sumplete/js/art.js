// Pixel-art rendering for Pixel Sumplete. 360x480 world units.

const PALETTE = {
  bg:        '#1d2240',
  card:      '#262d54',
  cell:      '#3a4274',
  cellSel:   '#7d8ed8',
  cellKeep:  '#54c47c',
  cellDelete:'#5a6188',
  digit:     '#f8f5e8',
  digitDim:  '#9aa6cc',
  rowT:      '#f7e69a',
  colT:      '#a8d84a',
  border:    '#0c1230',
  conflict:  '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function gridGeometry(n) {
  const cell = n === 4 ? 56 : n === 5 ? 48 : 42;
  const total = cell * n;
  const ox = ((360 - total - 36) / 2) | 0;        // leave room for right-side target column
  const oy = 76;
  return { cell, total, ox, oy };
}

function cellRect(n, x, y) {
  const g = gridGeometry(n);
  return { x: g.ox + x * g.cell, y: g.oy + y * g.cell, w: g.cell, h: g.cell };
}

function drawScene(ctx, p, marks, selected, conflicts) {
  const n = p.n;
  const g = gridGeometry(n);
  // Cells.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const r = cellRect(n, x, y);
    let fill = PALETTE.cell;
    if (marks[y][x] === 1)      fill = PALETTE.cellKeep;
    else if (marks[y][x] === 2) fill = PALETTE.cellDelete;
    if (selected && selected[0] === x && selected[1] === y) {
      // Outline the selected cell.
      fill = PALETTE.cellSel;
    }
    ctx.fillStyle = fill;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
      ctx.fillRect(r.x, r.y, r.w, r.h);
    }
    // Digit.
    ctx.fillStyle = marks[y][x] === 2 ? PALETTE.digitDim : PALETTE.digit;
    ctx.font = 'bold ' + ((g.cell * 0.55) | 0) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(p.grid[y][x]), r.x + r.w / 2, r.y + r.h / 2 + 1);
    if (marks[y][x] === 2) {
      // Strike-through.
      ctx.strokeStyle = PALETTE.cellDelete;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(r.x + 6, r.y + r.h - 6); ctx.lineTo(r.x + r.w - 6, r.y + 6);
      ctx.stroke();
    }
    // Cell border.
    ctx.strokeStyle = PALETTE.border;
    ctx.lineWidth = 1;
    ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
  }
  // Row targets on the right.
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let r = 0; r < n; r++) {
    const py = g.oy + r * g.cell + g.cell / 2;
    ctx.fillStyle = PALETTE.card;
    ctx.fillRect(g.ox + g.total + 6, g.oy + r * g.cell + 4, 28, g.cell - 8);
    ctx.fillStyle = PALETTE.rowT;
    ctx.fillText(String(p.rowT[r]), g.ox + g.total + 20, py + 1);
  }
  // Column targets on top.
  for (let c = 0; c < n; c++) {
    const px = g.ox + c * g.cell + g.cell / 2;
    ctx.fillStyle = PALETTE.card;
    ctx.fillRect(g.ox + c * g.cell + 4, g.oy - 28, g.cell - 8, 22);
    ctx.fillStyle = PALETTE.colT;
    ctx.fillText(String(p.colT[c]), px, g.oy - 16);
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
  const decided = marks.flat().filter(v => v !== 0).length;
  ctx.fillText(decided + '/' + (p.n * p.n), 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
