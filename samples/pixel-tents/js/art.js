// Pixel Tents - rendering.

const COL = {
  cellA: '#cfe8b8', cellB: '#bfd8a8', cellEdge: '#7a8d68',
  treeTrunk: '#7a4a26', treeLeaf: '#3a7e3a', treeLeafDark: '#2a5e2a',
  tent: '#e87a3a', tentDark: '#a04018', tentPole: '#36241a',
  tentBad: '#ff4a4a',
  grassX: '#5a6e4a',
  clueBg: '#1e2a18', clueText: '#ffd86b',
  clueDone: '#5fd36e',
  ok: '#5fd36e',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#3a6e3c');
  g.addColorStop(1, '#0e1f12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  // grid + 1 clue cell on right + 1 clue cell on bottom
  const cell = Math.min(42, Math.floor(312 / (n + 1)));
  const span = cell * (n + 1);
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(110 + (300 - span) / 2) };
}

function tileXY(geom, r, c) {
  return { x: geom.gx + c * geom.cell, y: geom.gy + r * geom.cell };
}

function drawBoard(ctx, pz, geom, cells, ev) {
  const { cell } = geom;
  const n = pz.n;
  // grid cells
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const p = tileXY(geom, r, c);
      ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
      ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
      ctx.strokeStyle = COL.cellEdge;
      ctx.lineWidth = 1;
      ctx.strokeRect(p.x + 0.5, p.y + 0.5, cell - 1, cell - 1);
      const i = r * n + c;
      if (pz.fixed[i]) drawTree(ctx, p.x, p.y, cell);
      else if (cells[i] === TENT) drawTent(ctx, p.x, p.y, cell, ev.bad.has(i));
      else if (cells[i] === GRASS) drawGrass(ctx, p.x, p.y, cell);
    }
  }
  // right column - row counts
  for (let r = 0; r < n; r++) {
    const p = tileXY(geom, r, n);
    drawClue(ctx, p.x, p.y, cell, pz.rowCnt[r], ev.rowUsed[r] === pz.rowCnt[r]);
  }
  // bottom row - col counts
  for (let c = 0; c < n; c++) {
    const p = tileXY(geom, n, c);
    drawClue(ctx, p.x, p.y, cell, pz.colCnt[c], ev.colUsed[c] === pz.colCnt[c]);
  }
}

function drawTree(ctx, x, y, cell) {
  const cx = x + cell / 2, cy = y + cell / 2;
  ctx.fillStyle = COL.treeTrunk;
  ctx.fillRect(cx - 3, cy + 2, 6, cell * 0.34);
  ctx.fillStyle = COL.treeLeafDark;
  ctx.beginPath();
  ctx.arc(cx, cy - 2, cell * 0.32, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = COL.treeLeaf;
  ctx.beginPath();
  ctx.arc(cx - 2, cy - 4, cell * 0.28, 0, Math.PI * 2);
  ctx.fill();
}

function drawTent(ctx, x, y, cell, bad) {
  const cx = x + cell / 2, cy = y + cell / 2;
  ctx.fillStyle = bad ? COL.tentBad : COL.tent;
  ctx.beginPath();
  ctx.moveTo(cx, cy - cell * 0.32);
  ctx.lineTo(cx + cell * 0.36, cy + cell * 0.28);
  ctx.lineTo(cx - cell * 0.36, cy + cell * 0.28);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = COL.tentDark;
  ctx.beginPath();
  ctx.moveTo(cx, cy - cell * 0.32);
  ctx.lineTo(cx + cell * 0.04, cy + cell * 0.28);
  ctx.lineTo(cx - cell * 0.04, cy + cell * 0.28);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = COL.tentPole;
  ctx.fillRect(cx - 1, cy - cell * 0.36, 2, 4);
}

function drawGrass(ctx, x, y, cell) {
  ctx.strokeStyle = COL.grassX;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(x + cell * 0.32, y + cell * 0.32);
  ctx.lineTo(x + cell * 0.68, y + cell * 0.68);
  ctx.moveTo(x + cell * 0.68, y + cell * 0.32);
  ctx.lineTo(x + cell * 0.32, y + cell * 0.68);
  ctx.stroke();
}

function drawClue(ctx, x, y, cell, n, done) {
  ctx.fillStyle = COL.clueBg;
  ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
  ctx.fillStyle = done ? COL.clueDone : COL.clueText;
  ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(n), x + cell / 2, y + cell / 2 + 1);
}
