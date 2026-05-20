// Pixel-art rendering for Pixel KenKen. 360x480 world units.

const PALETTE = {
  bg:       '#1d2240',
  card:     '#262d54',
  cell:     '#3a4274',
  cellSel:  '#7d8ed8',
  cellPeer: '#4c5996',
  digit:    '#f8f5e8',
  cageLabel:'#f7e69a',
  border:   '#0c1230',
  cageEdge: '#f8f5e8',
  conflict: '#e8554f',
  hud:      '#0d1228',
  hudText:  '#f8f5e8',
  hudDim:   '#9aa6cc',
  ok:       '#54c47c',
};

function gridGeometry(n) {
  const cell = n === 4 ? 56 : n === 5 ? 48 : 42;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 70;
  return { cell, total, ox, oy };
}

function cellRect(n, x, y) {
  const g = gridGeometry(n);
  return { x: g.ox + x * g.cell, y: g.oy + y * g.cell, w: g.cell, h: g.cell };
}

function drawScene(ctx, p, marks, selected, conflicts) {
  const n = p.n;
  const g = gridGeometry(n);
  // Cell-to-cage mapping.
  const cellCage = new Array(n * n);
  p.cages.forEach((cg, i) => cg.cells.forEach(c => cellCage[c] = i));
  // Cells.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const r = cellRect(n, x, y);
    let fill = PALETTE.cell;
    if (selected && selected[0] === x && selected[1] === y) fill = PALETTE.cellSel;
    else if (selected && (selected[0] === x || selected[1] === y)) fill = PALETTE.cellPeer;
    ctx.fillStyle = fill;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
      ctx.fillRect(r.x, r.y, r.w, r.h);
    }
    const v = marks[y][x];
    if (v) {
      ctx.fillStyle = PALETTE.digit;
      ctx.font = 'bold ' + ((g.cell * 0.55) | 0) + 'px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(v), r.x + r.w / 2, r.y + r.h / 2 + 2);
    }
  }
  // Thin cell borders.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= n; i++) {
    ctx.beginPath(); ctx.moveTo(g.ox + i * g.cell + 0.5, g.oy); ctx.lineTo(g.ox + i * g.cell + 0.5, g.oy + g.total); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(g.ox, g.oy + i * g.cell + 0.5); ctx.lineTo(g.ox + g.total, g.oy + i * g.cell + 0.5); ctx.stroke();
  }
  // Thick cage borders.
  ctx.strokeStyle = PALETTE.cageEdge;
  ctx.lineWidth = 2;
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const r = cellRect(n, x, y);
    const cg = cellCage[i];
    if (x === n - 1 || cellCage[y * n + x + 1] !== cg) {
      ctx.beginPath(); ctx.moveTo(r.x + r.w + 0.5, r.y); ctx.lineTo(r.x + r.w + 0.5, r.y + r.h); ctx.stroke();
    }
    if (y === n - 1 || cellCage[(y + 1) * n + x] !== cg) {
      ctx.beginPath(); ctx.moveTo(r.x, r.y + r.h + 0.5); ctx.lineTo(r.x + r.w, r.y + r.h + 0.5); ctx.stroke();
    }
    if (y === 0) { ctx.beginPath(); ctx.moveTo(r.x, r.y + 0.5); ctx.lineTo(r.x + r.w, r.y + 0.5); ctx.stroke(); }
    if (x === 0) { ctx.beginPath(); ctx.moveTo(r.x + 0.5, r.y); ctx.lineTo(r.x + 0.5, r.y + r.h); ctx.stroke(); }
  }
  ctx.strokeRect(g.ox, g.oy, g.total, g.total);
  // Cage label drawn in the top-left cell of each cage.
  ctx.fillStyle = PALETTE.cageLabel;
  ctx.font = 'bold ' + ((g.cell * 0.22) | 0) + 'px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  for (const cg of p.cages) {
    // Top-left cell = min by (y*n + x).
    const head = cg.cells.slice().sort((a, b) => a - b)[0];
    const hy = (head / n) | 0, hx = head % n;
    const r = cellRect(n, hx, hy);
    const label = cg.op === '=' ? String(cg.target) : (cg.target + cg.op);
    ctx.fillText(label, r.x + 3, r.y + 2);
  }
}

function drawNumberPad(ctx, n, lang, padHits) {
  const top = 388, h = 60;
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(8, top, 344, h);
  padHits.length = 0;
  const slots = n + 1;
  const bw = ((344 - 8) / slots) | 0;
  const bh = h - 8;
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 0; i < n; i++) {
    const bx = 12 + i * bw;
    const by = top + 4;
    ctx.fillStyle = '#54c47c';
    ctx.fillRect(bx, by, bw - 4, bh);
    ctx.fillStyle = PALETTE.hudText;
    ctx.fillText(String(i + 1), bx + (bw - 4) / 2, by + bh / 2 + 1);
    padHits.push({ kind: 'digit', v: i + 1, x: bx, y: by, w: bw - 4, h: bh });
  }
  const ex = 12 + n * bw;
  ctx.fillStyle = '#a05050';
  ctx.fillRect(ex, top + 4, bw - 4, bh);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 10px monospace';
  ctx.fillText(t(lang, 'erase'), ex + (bw - 4) / 2, top + 4 + bh / 2 + 1);
  padHits.push({ kind: 'erase', x: ex, y: top + 4, w: bw - 4, h: bh });
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
  const filled = marks.flat().filter(v => v).length;
  ctx.fillText(filled + '/' + (p.n * p.n), 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
