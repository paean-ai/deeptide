// Pixel Hitori - rendering.

const COL = {
  cellA: '#f2ead5', cellB: '#e6dec2', cellEdge: '#897e5e',
  text: '#22202a',
  shaded: '#22202a', shadedEdge: '#0a0a14', shadedText: '#22202a',
  marked: '#5fb070',
  bad: '#ff5a5a',
  hint: '#ffd86b',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#3a3a52');
  g.addColorStop(1, '#15151f');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  const cell = Math.min(56, Math.floor(312 / n));
  const span = cell * n;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(120 + (300 - span) / 2) };
}
function cellXY(g, n, r, c) {
  return { x: g.gx + c * g.cell, y: g.gy + r * g.cell };
}

function drawBoard(ctx, pz, geom, cells, ev) {
  const { cell } = geom;
  const n = pz.n;
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const i = r * n + c;
      const p = cellXY(geom, n, r, c);
      const v = cells[i];
      const isBad = ev.bad.has(i);
      if (v === SHADED) {
        ctx.fillStyle = isBad ? COL.bad : COL.shaded;
        ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
        ctx.strokeStyle = COL.shadedEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(p.x + 2.5, p.y + 2.5, cell - 5, cell - 5);
      } else {
        ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
        ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
        ctx.strokeStyle = COL.cellEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(p.x + 1.5, p.y + 1.5, cell - 3, cell - 3);
        // marked-open: small green corner mark
        if (v === MARKED_OPEN) {
          ctx.fillStyle = COL.marked;
          ctx.fillRect(p.x + 3, p.y + 3, 6, 6);
        }
        // number
        ctx.fillStyle = isBad ? COL.bad : COL.text;
        ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(String(pz.grid[i]), p.x + cell / 2, p.y + cell / 2 + 1);
      }
    }
  }
  if (ev.solved) {
    ctx.strokeStyle = COL.marked;
    ctx.lineWidth = 2;
    ctx.strokeRect(geom.gx - 2, geom.gy - 2, cell * n + 4, cell * n + 4);
  }
}
