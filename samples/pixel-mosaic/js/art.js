// Pixel Mosaic - rendering.

const COL = {
  cellA: '#d6d2c4', cellB: '#c9c4b2', cellEdge: '#8a8470',
  clue: '#22262e',
  fill: '#22262e',
  empty: '#7a7568',
  emptyMark: '#22262e',
  bad: '#ff6e7a',
  won: '#5fd36e',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#3a4456');
  g.addColorStop(1, '#16192a');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom() {
  const cell = 30;
  const span = cell * N;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(110 + (300 - span) / 2) };
}

function drawBoard(ctx, pz, geom, cells, ev, color) {
  const { cell, gx, gy } = geom;
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      const i = r * N + c;
      const x = gx + c * cell, y = gy + r * cell;
      const v = cells[i];
      const isBad = ev.bad.has(r + ',' + c);
      if (v === FILLED) {
        ctx.fillStyle = ev.solved ? (color || COL.fill) : COL.fill;
        ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
      } else {
        ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
        ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
      }
      ctx.strokeStyle = COL.cellEdge;
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
      if (v === EMPTY) {
        ctx.strokeStyle = COL.emptyMark;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(x + cell * 0.3, y + cell * 0.3);
        ctx.lineTo(x + cell * 0.7, y + cell * 0.7);
        ctx.moveTo(x + cell * 0.7, y + cell * 0.3);
        ctx.lineTo(x + cell * 0.3, y + cell * 0.7);
        ctx.stroke();
      }
      if (!ev.solved) {
        const clue = ev.clues[r][c];
        ctx.fillStyle = isBad ? COL.bad : (v === FILLED ? '#e8e2c8' : COL.clue);
        ctx.font = 'bold ' + Math.round(cell * 0.48) + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(String(clue), x + cell / 2, y + cell / 2 + 1);
      }
    }
  }
  if (ev.solved && color) {
    ctx.strokeStyle = COL.won;
    ctx.lineWidth = 2;
    ctx.strokeRect(gx - 2, gy - 2, cell * N + 4, cell * N + 4);
  }
}
