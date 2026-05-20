// Pixel Kuromasu - rendering.

const COL = {
  cellA: '#dde4f0', cellB: '#cdd4e0', cellEdge: '#7a8398',
  hintBg: '#fff3c4', hintText: '#3a3018',
  blackCell: '#1a1d2c', blackEdge: '#0a0c14',
  whiteMark: '#5fb070',
  bad: '#ff5a5a',
  ok: '#5fd36e',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#3a4458');
  g.addColorStop(1, '#0c1020');
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

function tileXY(geom, r, c) {
  return { x: geom.gx + c * geom.cell, y: geom.gy + r * geom.cell };
}

function drawBoard(ctx, pz, geom, marks, ev) {
  const { cell } = geom;
  const n = pz.n;
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const i = r * n + c;
      const p = tileXY(geom, r, c);
      const hintVal = pz.hints[i];
      const isBad = ev.bad.has(i);
      const v = marks[i];
      if (v === PB_BLACK) {
        ctx.fillStyle = isBad ? COL.bad : COL.blackCell;
        ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
        ctx.strokeStyle = COL.blackEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(p.x + 1.5, p.y + 1.5, cell - 3, cell - 3);
        continue;
      }
      if (hintVal !== undefined) {
        ctx.fillStyle = COL.hintBg;
        ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
        ctx.strokeStyle = COL.cellEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(p.x + 1.5, p.y + 1.5, cell - 3, cell - 3);
        ctx.fillStyle = isBad ? COL.bad : COL.hintText;
        ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(String(hintVal), p.x + cell / 2, p.y + cell / 2 + 1);
        continue;
      }
      // blank cell
      ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
      ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
      ctx.strokeStyle = COL.cellEdge;
      ctx.lineWidth = 1;
      ctx.strokeRect(p.x + 1.5, p.y + 1.5, cell - 3, cell - 3);
      if (v === PB_WHITE) {
        ctx.fillStyle = COL.whiteMark;
        ctx.fillRect(p.x + cell * 0.4, p.y + cell * 0.4, cell * 0.2, cell * 0.2);
      }
    }
  }
  if (ev.solved) {
    ctx.strokeStyle = COL.ok;
    ctx.lineWidth = 2;
    ctx.strokeRect(geom.gx - 2, geom.gy - 2, cell * n + 4, cell * n + 4);
  }
}
