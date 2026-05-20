// Pixel Shikaku - rendering.

const COL = {
  cellA: '#1d2740', cellB: '#172033', grid: '#0a1020',
  clueBg: '#0a1020', clueText: '#ffd86b',
  preview: 'rgba(255, 216, 107, 0.32)', previewEdge: '#ffd86b',
  bad: 'rgba(255, 110, 122, 0.34)', badEdge: '#ff6e7a',
  won: '#5fd36e',
};

const RECT_COLORS = [
  '#6cd6c8', '#ffae6b', '#a7b8ff', '#ff8fc0', '#9ce06b',
  '#e8c984', '#7fdbff', '#c89bff', '#ff7d52', '#5fc8e8',
];

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#22304a');
  g.addColorStop(1, '#0a0e1c');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(pz) {
  const cell = Math.min(44, Math.floor(312 / pz.w), Math.floor(304 / pz.h));
  const sw = cell * pz.w, sh = cell * pz.h;
  return { cell, gx: Math.round((VW - sw) / 2), gy: Math.round(110 + (300 - sh) / 2) };
}
function cellXY(g, r, c) { return { x: g.gx + c * g.cell, y: g.gy + r * g.cell }; }

function drawBoard(ctx, pz, geom, rects, preview, previewValid) {
  const { cell } = geom;
  // cells
  for (let r = 0; r < pz.h; r++) {
    for (let c = 0; c < pz.w; c++) {
      const p = cellXY(geom, r, c);
      ctx.fillStyle = (r + c) % 2 ? COL.cellA : COL.cellB;
      ctx.fillRect(p.x, p.y, cell, cell);
      ctx.strokeStyle = COL.grid;
      ctx.lineWidth = 1;
      ctx.strokeRect(p.x + 0.5, p.y + 0.5, cell - 1, cell - 1);
    }
  }
  // placed rectangles
  rects.forEach((rec, i) => {
    if (!rec) return;
    const p = cellXY(geom, rec.r, rec.c);
    const c = RECT_COLORS[i % RECT_COLORS.length];
    ctx.fillStyle = withAlpha(c, 0.28);
    ctx.fillRect(p.x + 1, p.y + 1, rec.rw * cell - 2, rec.rh * cell - 2);
    ctx.strokeStyle = c;
    ctx.lineWidth = 3;
    ctx.strokeRect(p.x + 2, p.y + 2, rec.rw * cell - 4, rec.rh * cell - 4);
  });
  // clue circles
  for (const cl of pz.clues) {
    const p = cellXY(geom, cl.r, cl.c);
    ctx.fillStyle = COL.clueBg;
    ctx.beginPath();
    ctx.arc(p.x + cell / 2, p.y + cell / 2, cell * 0.34, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = COL.clueText;
    ctx.lineWidth = 1.5;
    ctx.stroke();
    ctx.fillStyle = COL.clueText;
    ctx.font = 'bold ' + Math.round(cell * 0.42) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(cl.n), p.x + cell / 2, p.y + cell / 2 + 1);
  }
  // preview rectangle
  if (preview) {
    const p = cellXY(geom, preview.r, preview.c);
    ctx.fillStyle = previewValid ? COL.preview : COL.bad;
    ctx.fillRect(p.x + 1, p.y + 1, preview.rw * cell - 2, preview.rh * cell - 2);
    ctx.strokeStyle = previewValid ? COL.previewEdge : COL.badEdge;
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 4]);
    ctx.strokeRect(p.x + 2, p.y + 2, preview.rw * cell - 4, preview.rh * cell - 4);
    ctx.setLineDash([]);
  }
}

function withAlpha(hex, a) {
  const m = hex.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i);
  if (!m) return hex;
  const r = parseInt(m[1], 16), g = parseInt(m[2], 16), b = parseInt(m[3], 16);
  return `rgba(${r},${g},${b},${a})`;
}
