// Pixel Light Up - rendering.

const COL = {
  unlit: '#1c2030', lit: '#3a3520', litEdge: '#5a5230',
  wall: '#0c0e16', wallEdge: '#2a3048', wallText: '#cdd4e6',
  grid: '#11141f',
  bulb: '#ffd34e', bulbBad: '#ff6e7a',
  ok: '#5fd36e', over: '#ff6e7a',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#181c2c');
  g.addColorStop(1, '#0b0d16');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(pz) {
  const cell = Math.min(40, Math.floor(320 / pz.w), Math.floor(300 / pz.h));
  return {
    cell,
    gx: Math.round((VW - cell * pz.w) / 2),
    gy: Math.round(100 + (300 - cell * pz.h) / 2),
  };
}

function drawBoard(ctx, pz, geom, ev, bulbs) {
  const { cell, gx, gy } = geom;
  for (let r = 0; r < pz.h; r++) {
    for (let c = 0; c < pz.w; c++) {
      const x = gx + c * cell, y = gy + r * cell;
      if (pz.wall[r][c]) {
        ctx.fillStyle = COL.wall;
        ctx.fillRect(x, y, cell, cell);
        ctx.strokeStyle = COL.wallEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
        const n = pz.number[r][c];
        if (n >= 0) {
          const got = ev.wallState[r + ',' + c] || 0;
          ctx.fillStyle = got === n ? COL.ok : got > n ? COL.over : COL.wallText;
          ctx.font = 'bold ' + Math.round(cell * 0.5) + 'px monospace';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(String(n), x + cell / 2, y + cell / 2 + 1);
        }
        continue;
      }
      const key = r + ',' + c;
      const isLit = ev.lit[key];
      ctx.fillStyle = isLit ? COL.lit : COL.unlit;
      ctx.fillRect(x, y, cell, cell);
      ctx.strokeStyle = COL.grid;
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
      if (bulbs.has(key)) drawBulb(ctx, x, y, cell, ev.conflict.has(key));
    }
  }
}

function drawBulb(ctx, x, y, cell, bad) {
  const cx = x + cell / 2, cy = y + cell / 2, r = cell * 0.3;
  if (!bad) {
    ctx.fillStyle = 'rgba(255,211,78,0.28)';
    ctx.beginPath();
    ctx.arc(cx, cy, r * 1.9, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = bad ? COL.bulbBad : COL.bulb;
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = bad ? '#5a1f26' : '#7a5a14';
  ctx.fillRect(cx - r * 0.4, cy + r * 0.7, r * 0.8, r * 0.5);
}
