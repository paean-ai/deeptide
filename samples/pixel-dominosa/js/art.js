// Pixel Dominosa - rendering.

const COL = {
  tile: '#f3ead4', tileEdge: '#b9a878', pip: '#2b2418',
  domino: '#caa24a', dominoFill: 'rgba(202,162,74,0.26)',
  dup: '#ff6e7a', dupFill: 'rgba(255,110,122,0.22)',
};

// pip dot positions on a 3x3 grid (col, row in 0..2)
const PIPS = {
  0: [], 1: [[1, 1]], 2: [[0, 0], [2, 2]],
  3: [[0, 0], [1, 1], [2, 2]],
  4: [[0, 0], [2, 0], [0, 2], [2, 2]],
  5: [[0, 0], [2, 0], [1, 1], [0, 2], [2, 2]],
  6: [[0, 0], [2, 0], [0, 1], [2, 1], [0, 2], [2, 2]],
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1f3326');
  g.addColorStop(1, '#0c160f');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(pz) {
  const cell = Math.min(46, Math.floor(312 / pz.cols), Math.floor(286 / pz.rows));
  return {
    cell,
    gx: Math.round((VW - cell * pz.cols) / 2),
    gy: Math.round(100 + (288 - cell * pz.rows) / 2),
  };
}
function cellXY(geom, idx, cols) {
  return { x: geom.gx + (idx % cols) * geom.cell, y: geom.gy + ((idx / cols) | 0) * geom.cell };
}

function drawBoard(ctx, pz, geom, ev, dominoes) {
  const { cell } = geom;
  // tiles + pips
  for (let i = 0; i < pz.pips.length; i++) {
    const p = cellXY(geom, i, pz.cols);
    ctx.fillStyle = COL.tile;
    ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
    ctx.strokeStyle = COL.tileEdge;
    ctx.lineWidth = 1;
    ctx.strokeRect(p.x + 2.5, p.y + 2.5, cell - 5, cell - 5);
    drawPips(ctx, p.x, p.y, cell, pz.pips[i]);
  }
  // domino capsules
  dominoes.forEach((d, i) => {
    const a = cellXY(geom, d[0], pz.cols), b = cellXY(geom, d[1], pz.cols);
    const bad = ev.dup.has(i);
    const x = Math.min(a.x, b.x) + 4, y = Math.min(a.y, b.y) + 4;
    const w = Math.abs(a.x - b.x) + cell - 8, h = Math.abs(a.y - b.y) + cell - 8;
    ctx.fillStyle = bad ? COL.dupFill : COL.dominoFill;
    roundRect(ctx, x, y, w, h, 7);
    ctx.fill();
    ctx.strokeStyle = bad ? COL.dup : COL.domino;
    ctx.lineWidth = 3;
    roundRect(ctx, x, y, w, h, 7);
    ctx.stroke();
  });
}

function drawPips(ctx, x, y, cell, value) {
  ctx.fillStyle = COL.pip;
  const r = Math.max(2, cell * 0.075);
  const pad = cell * 0.26, span = cell - 2 * pad;
  for (const [gc, gr] of PIPS[value] || []) {
    const cx = x + pad + span * (gc / 2);
    const cy = y + pad + span * (gr / 2);
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.fill();
  }
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
