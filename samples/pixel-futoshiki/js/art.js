// Pixel-art rendering for Pixel Futoshiki. 360x480 world units.

const PALETTE = {
  bg:        '#1d2240',
  card:      '#262d54',
  cell:      '#3a4274',
  cellSel:   '#7d8ed8',
  cellPeer:  '#4c5996',
  digit:     '#f8f5e8',
  digitDim:  '#9aa6cc',
  signGT:    '#f7e69a',
  signLT:    '#a8d84a',
  border:    '#0c1230',
  outerBor:  '#f8f5e8',
  conflict:  '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function gridGeometry(n) {
  // Leave gaps between cells for the inequality signs. Cell size 40 with 14px gaps gives:
  //   for n=4: 4*40 + 3*14 = 202; for n=5: 5*40 + 4*14 = 256; for n=6: 6*40 + 5*14 = 310
  // Use smaller cells for bigger grids.
  const cell = n <= 4 ? 50 : n === 5 ? 44 : 38;
  const gap  = n <= 4 ? 16 : n === 5 ? 14 : 12;
  const total = n * cell + (n - 1) * gap;
  const ox = ((360 - total) / 2) | 0;
  const oy = 70;
  return { cell, gap, total, ox, oy };
}

function cellRect(n, x, y) {
  const g = gridGeometry(n);
  return { x: g.ox + x * (g.cell + g.gap), y: g.oy + y * (g.cell + g.gap), w: g.cell, h: g.cell };
}

function drawScene(ctx, p, marks, selected, conflicts) {
  const n = p.n;
  const g = gridGeometry(n);
  // Cells.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const r = cellRect(n, x, y);
    let fill = PALETTE.cell;
    if (selected && selected[0] === x && selected[1] === y) fill = PALETTE.cellSel;
    else if (selected && (selected[0] === x || selected[1] === y)) fill = PALETTE.cellPeer;
    ctx.fillStyle = fill;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    if (conflicts && conflicts.has(y * n + x)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
      ctx.fillRect(r.x, r.y, r.w, r.h);
    }
    const v = marks[y][x];
    if (v) {
      ctx.fillStyle = PALETTE.digit;
      ctx.font = 'bold ' + ((g.cell * 0.55) | 0) + 'px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(v), r.x + r.w / 2, r.y + r.h / 2 + 1);
    }
    // Cell border (thin).
    ctx.strokeStyle = PALETTE.border;
    ctx.lineWidth = 1;
    ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
  }
  // Constraints (> / <) drawn between cells.
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.font = 'bold ' + ((g.gap + 2) | 0) + 'px monospace';
  for (const cn of p.constraints) {
    const a = cellRect(n, cn.ax, cn.ay);
    const b = cellRect(n, cn.bx, cn.by);
    let mx, my, sym;
    if (cn.ax === cn.bx) {
      // Vertical pair: a above b.
      mx = a.x + a.w / 2;
      my = (a.y + a.h + b.y) / 2;
      // op is "a OP b"; if op is >, a > b means a is bigger (above), drawn as 'v' (down-arrow / wider top)
      sym = cn.op === '>' ? '⌄' : '⌃';
      // Use ascii fallback for stable rendering.
      sym = cn.op === '>' ? 'v' : '^';
    } else {
      mx = (a.x + a.w + b.x) / 2;
      my = a.y + a.h / 2;
      sym = cn.op;
    }
    ctx.fillStyle = cn.op === '>' ? PALETTE.signGT : PALETTE.signLT;
    ctx.fillText(sym, mx, my + 1);
  }
}

function drawNumberPad(ctx, n, lang, padHits) {
  const top = 388, h = 60;
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(8, top, 344, h);
  padHits.length = 0;
  const slots = n + 1;                  // n digits + erase
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
  ctx.fillText(filled + '/' + (p.n * p.n) + ' FILLED', 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
