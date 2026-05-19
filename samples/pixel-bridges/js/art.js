// Pixel Bridges - rendering.

const COL = {
  sea1: '#16243c', sea2: '#0c1626',
  island: '#e7c98a', islandEdge: '#7a5f33', islandText: '#2a1f0c',
  ok: '#5fd36e', over: '#ff6e7a', sel: '#ffe07a',
  bridge: '#cde3ef', bridgeHi: '#ffe07a',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, COL.sea1);
  g.addColorStop(1, COL.sea2);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(pz) {
  const cell = Math.min(42, Math.floor(320 / pz.w), Math.floor(300 / pz.h));
  return {
    cell,
    gx: Math.round((VW - cell * pz.w) / 2 + cell / 2),
    gy: Math.round(98 + (300 - cell * pz.h) / 2 + cell / 2),
  };
}
function islandXY(geom, is) {
  return { x: geom.gx + is.c * geom.cell, y: geom.gy + is.r * geom.cell };
}

function drawBoard(ctx, pz, geom, st) {
  // bridges
  for (let i = 0; i < pz.edges.length; i++) {
    const v = st.bridges[i] || 0;
    if (v === 0) continue;
    drawBridge(ctx, geom, pz.islands[pz.edges[i].a], pz.islands[pz.edges[i].b], v, i === st.hover);
  }
  // islands
  const r = Math.round(geom.cell * 0.4);
  for (let i = 0; i < pz.islands.length; i++) {
    const is = pz.islands[i];
    const p = islandXY(geom, is);
    const need = is.n, have = st.counts[i] || 0;
    ctx.fillStyle = COL.island;
    ctx.beginPath();
    ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.lineWidth = 3;
    ctx.strokeStyle = have === need ? COL.ok : have > need ? COL.over : COL.islandEdge;
    ctx.stroke();
    ctx.fillStyle = COL.islandText;
    ctx.font = 'bold ' + Math.round(geom.cell * 0.5) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(need), p.x, p.y + 1);
  }
}

function drawBridge(ctx, geom, a, b, v, hi) {
  const pa = islandXY(geom, a), pb = islandXY(geom, b);
  const horiz = a.r === b.r;
  ctx.strokeStyle = hi ? COL.bridgeHi : COL.bridge;
  ctx.lineWidth = 3;
  const off = v === 2 ? 4 : 0;
  for (let k = 0; k < v; k++) {
    const d = v === 2 ? (k === 0 ? -off : off) : 0;
    ctx.beginPath();
    if (horiz) { ctx.moveTo(pa.x, pa.y + d); ctx.lineTo(pb.x, pb.y + d); }
    else { ctx.moveTo(pa.x + d, pa.y); ctx.lineTo(pb.x + d, pb.y); }
    ctx.stroke();
  }
}
