// Pixel Trail - rendering.

const COL = {
  cellA: '#1d2336', cellB: '#181d2e', cellEdge: '#2c3450',
  clue: '#ffd86b', clueText: '#1a1606',
  trail: '#6cd6c8', trailText: '#0a2522',
  trailLine: '#a7ffea', tip: '#ffe07a',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1e2a44');
  g.addColorStop(1, '#0a0f1c');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  const cell = Math.min(50, Math.floor(312 / n));
  const span = cell * n;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(120 + (300 - span) / 2) };
}
function tileXY(g, n, idx) {
  return { x: g.gx + (idx % n) * g.cell, y: g.gy + ((idx / n) | 0) * g.cell };
}

function drawBoard(ctx, pz, geom, pathSoFar) {
  const { cell } = geom;
  const onPath = {};
  pathSoFar.forEach((c, k) => { onPath[c] = k + 1; });
  // cells
  for (let i = 0; i < pz.N; i++) {
    const p = tileXY(geom, pz.n, i);
    const r = (i / pz.n) | 0, c = i % pz.n;
    const isClue = pz.revealed[i] !== undefined;
    const num = onPath[i] || (isClue ? pz.revealed[i] : 0);
    ctx.fillStyle = onPath[i]
      ? COL.trail
      : (r + c) % 2 ? COL.cellA : COL.cellB;
    ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
    ctx.strokeStyle = COL.cellEdge;
    ctx.lineWidth = 1;
    ctx.strokeRect(p.x + 1.5, p.y + 1.5, cell - 3, cell - 3);
    if (isClue && !onPath[i]) {
      // clue ring
      ctx.strokeStyle = COL.clue;
      ctx.lineWidth = 2;
      ctx.strokeRect(p.x + 4, p.y + 4, cell - 8, cell - 8);
    }
    if (num) {
      ctx.fillStyle = onPath[i] ? COL.trailText
        : isClue ? COL.clue : '#fff';
      ctx.font = 'bold ' + Math.round(cell * 0.42) + 'px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(num), p.x + cell / 2, p.y + cell / 2 + 1);
    }
  }
  // path lines
  if (pathSoFar.length >= 2) {
    ctx.strokeStyle = COL.trailLine;
    ctx.lineWidth = 4;
    ctx.lineCap = 'round';
    ctx.beginPath();
    for (let k = 0; k < pathSoFar.length; k++) {
      const p = tileXY(geom, pz.n, pathSoFar[k]);
      const cx = p.x + cell / 2, cy = p.y + cell / 2;
      if (k === 0) ctx.moveTo(cx, cy); else ctx.lineTo(cx, cy);
    }
    ctx.stroke();
  }
  // tip marker
  const tip = pathSoFar[pathSoFar.length - 1];
  if (tip !== undefined) {
    const p = tileXY(geom, pz.n, tip);
    ctx.strokeStyle = COL.tip;
    ctx.lineWidth = 3;
    ctx.strokeRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
  }
}
