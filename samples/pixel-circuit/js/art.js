// Pixel Circuit - rendering.

const COL = {
  board: '#0e2616', boardEdge: '#1d4226', trace: '#2c4d34',
  tile: '#143c25', tileEdge: '#1d4226',
  off: '#3a8a52', on: '#a7ff6f', glow: 'rgba(167,255,111,0.5)',
  source: '#ffd23e', sourceDark: '#7a6010',
  leak: '#ff6e6e',
  node: '#5fd36e',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#0d2a17');
  g.addColorStop(1, '#06140b');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  const cell = Math.min(60, Math.floor(312 / n));
  const span = cell * n;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(120 + (296 - span) / 2) };
}
function tileRect(geom, i, n) {
  return { x: geom.gx + (i % n) * geom.cell, y: geom.gy + ((i / n) | 0) * geom.cell, s: geom.cell };
}

const DIR_VEC = [{ dr: -1, dc: 0 }, { dr: 0, dc: 1 }, { dr: 1, dc: 0 }, { dr: 0, dc: -1 }];

function drawBoard(ctx, pz, geom, ev, anim) {
  const { cell, gx, gy } = geom;
  // board background panel
  ctx.fillStyle = COL.board;
  ctx.fillRect(gx - 6, gy - 6, cell * pz.n + 12, cell * pz.n + 12);
  ctx.strokeStyle = COL.boardEdge;
  ctx.lineWidth = 2;
  ctx.strokeRect(gx - 5.5, gy - 5.5, cell * pz.n + 11, cell * pz.n + 11);
  for (let i = 0; i < pz.cells.length; i++) {
    drawTile(ctx, geom, pz, i, ev, anim && anim.idx === i ? anim : null);
  }
}

function drawTile(ctx, geom, pz, i, ev, anim) {
  const r = tileRect(geom, i, pz.n);
  // tile bed
  ctx.fillStyle = COL.tile;
  ctx.fillRect(r.x, r.y, r.s, r.s);
  ctx.strokeStyle = COL.tileEdge;
  ctx.lineWidth = 1;
  ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.s - 1, r.s - 1);
  const cell = pz.cells[i];
  // rotation extra for animation
  const animRot = anim ? -anim.t * (Math.PI / 2) : 0; // animate CW
  const cx = r.x + r.s / 2, cy = r.y + r.s / 2;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate(animRot);
  const eff = ev.eff[i];
  const powered = ev.powered[i];
  const colorMain = powered ? COL.on : COL.off;
  const colorBack = powered ? COL.glow : 'rgba(60,90,70,0.35)';
  // wire pieces
  const wireW = Math.max(5, r.s * 0.18);
  // back glow
  if (powered) {
    ctx.fillStyle = colorBack;
    ctx.beginPath();
    ctx.arc(0, 0, r.s * 0.5, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.strokeStyle = colorMain;
  ctx.lineWidth = wireW;
  ctx.lineCap = 'butt';
  for (let d = 0; d < 4; d++) {
    if (!(eff & (1 << d))) continue;
    const v = DIR_VEC[d];
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(v.dc * r.s * 0.5, v.dr * r.s * 0.5);
    ctx.stroke();
    // leak marker: a red cap at the end if neighbour doesn't match
    const nr = ((i / pz.n) | 0) + v.dr, nc = (i % pz.n) + v.dc;
    let leakEnd = false;
    if (nr < 0 || nc < 0 || nr >= pz.n || nc >= pz.n) leakEnd = true;
    else {
      const obit = 1 << ((d + 2) & 3);
      if (!(ev.eff[nr * pz.n + nc] & obit)) leakEnd = true;
    }
    if (leakEnd) {
      ctx.fillStyle = COL.leak;
      ctx.fillRect(v.dc * r.s * 0.42 - 4, v.dr * r.s * 0.42 - 4, 8, 8);
    }
  }
  // node dot at centre
  ctx.fillStyle = powered ? colorMain : '#2c4d34';
  ctx.beginPath();
  ctx.arc(0, 0, wireW * 0.62, 0, Math.PI * 2);
  ctx.fill();
  // source: golden core
  if (i === pz.source) {
    ctx.fillStyle = COL.source;
    ctx.beginPath();
    ctx.arc(0, 0, r.s * 0.22, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = COL.sourceDark;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
  ctx.restore();
}
