// Pixel Skyline - rendering.

const COL = {
  clueBg: '#1f253a', clueText: '#ffd86b',
  cellA: '#e7eef8', cellB: '#d5dceb', cellEdge: '#7e8aa6',
  text: '#1a2030',
  bad: '#ff6e7a',
  ok: '#5fd36e',
  selBorder: '#ffd86b',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#28304a');
  g.addColorStop(1, '#0c1020');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  // n+2 cells across (inner grid + 1 clue cell each side)
  const cell = Math.min(50, Math.floor(312 / (n + 2)));
  const span = cell * (n + 2);
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(120 + (300 - span) / 2) };
}

function tileXY(geom, r, c) {
  return { x: geom.gx + c * geom.cell, y: geom.gy + r * geom.cell };
}

function drawBoard(ctx, pz, geom, cells, ev, selected) {
  const { cell } = geom;
  const n = pz.n;
  // clue cells: top row, bottom row, left col, right col
  drawClueRow(ctx, geom, n, 0, 1, pz.clues.top);
  drawClueRow(ctx, geom, n, n + 1, 1, pz.clues.bottom);
  drawClueCol(ctx, geom, n, 0, 1, pz.clues.left);
  drawClueCol(ctx, geom, n, n + 1, 1, pz.clues.right);

  // inner cells (rows 1..n, cols 1..n)
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const i = r * n + c;
      const p = tileXY(geom, r + 1, c + 1);
      const v = cells[i];
      const bad = ev.bad.has(i);
      ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
      ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
      ctx.strokeStyle = COL.cellEdge;
      ctx.lineWidth = 1;
      ctx.strokeRect(p.x + 0.5, p.y + 0.5, cell - 1, cell - 1);
      if (selected === i) {
        ctx.strokeStyle = COL.selBorder;
        ctx.lineWidth = 2;
        ctx.strokeRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
      }
      if (v) {
        ctx.fillStyle = bad ? COL.bad : COL.text;
        ctx.font = 'bold ' + Math.round(cell * 0.55) + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(String(v), p.x + cell / 2, p.y + cell / 2 + 1);
      }
    }
  }
  if (ev.solved) {
    ctx.strokeStyle = COL.ok;
    ctx.lineWidth = 2;
    ctx.strokeRect(geom.gx + cell - 2, geom.gy + cell - 2, cell * n + 4, cell * n + 4);
  }
}

function drawClueRow(ctx, geom, n, row, colStart, arr) {
  const { cell } = geom;
  for (let i = 0; i < n; i++) {
    const p = tileXY(geom, row, colStart + i);
    ctx.fillStyle = COL.clueBg;
    ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
    ctx.fillStyle = COL.clueText;
    ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(arr[i]), p.x + cell / 2, p.y + cell / 2 + 1);
  }
}
function drawClueCol(ctx, geom, n, col, rowStart, arr) {
  const { cell } = geom;
  for (let i = 0; i < n; i++) {
    const p = tileXY(geom, rowStart + i, col);
    ctx.fillStyle = COL.clueBg;
    ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
    ctx.fillStyle = COL.clueText;
    ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(arr[i]), p.x + cell / 2, p.y + cell / 2 + 1);
  }
}
