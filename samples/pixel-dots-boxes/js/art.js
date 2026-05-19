// Pixel Dots & Boxes - rendering.

const COL = {
  player: '#5fd36e', playerDim: '#2f6e3a',
  ai: '#ff6e8a', aiDim: '#7a3344',
  dot: '#e6ebf5', edge: '#2a3350', edgeHi: '#586288',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#171c2e');
  g.addColorStop(1, '#0e1120');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

// board geometry shared by render + input
function boardGeom() {
  const cell = 52;
  const span = cell * B;
  return { cell, span, gx: Math.round((VW - span) / 2), gy: 138 };
}
function dotXY(g, col, row) {
  return { x: g.gx + col * g.cell, y: g.gy + row * g.cell };
}

function drawBoard(ctx, s, lastEdge) {
  const g = boardGeom();
  // claimed boxes
  for (let r = 0; r < B; r++) for (let c = 0; c < B; c++) {
    const o = s.boxes[r][c];
    if (!o) continue;
    const p = dotXY(g, c, r);
    ctx.fillStyle = o === PLAYER ? COL.playerDim : COL.aiDim;
    ctx.fillRect(p.x + 3, p.y + 3, g.cell - 6, g.cell - 6);
    ctx.fillStyle = o === PLAYER ? COL.player : COL.ai;
    ctx.font = 'bold 22px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(o === PLAYER ? '●' : '◆', p.x + g.cell / 2, p.y + g.cell / 2 + 1);
  }
  // edges
  const lk = lastEdge ? lastEdge.t + ':' + lastEdge.r + ':' + lastEdge.c : '';
  for (let r = 0; r <= B; r++) for (let c = 0; c < B; c++) {
    const a = dotXY(g, c, r);
    drawEdge(ctx, a.x, a.y, a.x + g.cell, a.y, s.h[r][c], lk === '0:' + r + ':' + c);
  }
  for (let r = 0; r < B; r++) for (let c = 0; c <= B; c++) {
    const a = dotXY(g, c, r);
    drawEdge(ctx, a.x, a.y, a.x, a.y + g.cell, s.v[r][c], lk === '1:' + r + ':' + c);
  }
  // dots
  for (let r = 0; r <= B; r++) for (let c = 0; c <= B; c++) {
    const p = dotXY(g, c, r);
    ctx.fillStyle = COL.dot;
    ctx.fillRect(p.x - 3, p.y - 3, 6, 6);
  }
}

function drawEdge(ctx, x1, y1, x2, y2, drawn, highlight) {
  ctx.strokeStyle = drawn ? (highlight ? '#ffe07a' : COL.edgeHi) : COL.edge;
  ctx.lineWidth = drawn ? 5 : 2;
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();
}
