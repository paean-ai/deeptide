// Pixel Glacier - rendering.

const COL = {
  iceA: '#cfe8f5', iceB: '#bcdcef', iceEdge: '#9fc4da',
  rock: '#3c4a5e', rockTop: '#52617a',
  exit: '#ffd76b', exitGlow: 'rgba(255,215,107,0.5)',
  player: '#e8554f', playerDark: '#a83730', playerEye: '#fff',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#243a52');
  g.addColorStop(1, '#0f1b2c');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.04)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  const cell = Math.min(48, Math.floor(312 / n));
  const span = cell * n;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(118 + (300 - span) / 2) };
}

function drawBoard(ctx, pz, geom, anim, clock) {
  const { cell, gx, gy } = geom;
  for (let r = 0; r < pz.n; r++) {
    for (let c = 0; c < pz.n; c++) {
      const t = pz.grid[r * pz.n + c];
      const x = gx + c * cell, y = gy + r * cell;
      if (t === ROCK) {
        ctx.fillStyle = COL.rock;
        ctx.fillRect(x, y, cell, cell);
        ctx.fillStyle = COL.rockTop;
        ctx.fillRect(x + 4, y + 4, cell - 8, cell - 10);
      } else {
        ctx.fillStyle = (r + c) % 2 ? COL.iceA : COL.iceB;
        ctx.fillRect(x, y, cell, cell);
        ctx.strokeStyle = COL.iceEdge;
        ctx.lineWidth = 1;
        ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
        // ice glint
        ctx.fillStyle = 'rgba(255,255,255,0.35)';
        ctx.fillRect(x + 5, y + 5, cell * 0.28, 2);
      }
      if (t === EXIT) drawExit(ctx, x, y, cell, clock);
    }
  }
  // player
  let pr = (pz.start / pz.n) | 0, pc = pz.start % pz.n;
  if (anim) { pr = anim.r; pc = anim.c; }
  drawPlayer(ctx, gx + pc * cell, gy + pr * cell, cell);
}

function drawExit(ctx, x, y, cell, clock) {
  const cx = x + cell / 2, cy = y + cell / 2;
  const pulse = 0.6 + 0.4 * Math.sin(clock * 3);
  ctx.fillStyle = COL.exitGlow;
  ctx.globalAlpha = pulse;
  ctx.beginPath();
  ctx.arc(cx, cy, cell * 0.42, 0, Math.PI * 2);
  ctx.fill();
  ctx.globalAlpha = 1;
  ctx.fillStyle = COL.exit;
  for (let ring = 2; ring >= 0; ring--) {
    ctx.globalAlpha = 0.4 + ring * 0.3;
    const rr = cell * (0.13 + ring * 0.08);
    ctx.beginPath();
    ctx.arc(cx, cy, rr, 0, Math.PI * 2);
    ctx.stroke();
    ctx.strokeStyle = COL.exit;
    ctx.lineWidth = 2;
  }
  ctx.globalAlpha = 1;
}

function drawPlayer(ctx, x, y, cell) {
  const m = cell * 0.16, sz = cell - m * 2;
  ctx.fillStyle = COL.playerDark;
  ctx.fillRect(x + m, y + m + sz * 0.78, sz, sz * 0.22);
  ctx.fillStyle = COL.player;
  ctx.fillRect(x + m, y + m, sz, sz * 0.82);
  ctx.fillStyle = COL.playerEye;
  ctx.fillRect(x + m + sz * 0.2, y + m + sz * 0.26, sz * 0.2, sz * 0.2);
  ctx.fillRect(x + m + sz * 0.58, y + m + sz * 0.26, sz * 0.2, sz * 0.2);
  ctx.fillStyle = '#10101a';
  ctx.fillRect(x + m + sz * 0.27, y + m + sz * 0.32, sz * 0.09, sz * 0.09);
  ctx.fillRect(x + m + sz * 0.65, y + m + sz * 0.32, sz * 0.09, sz * 0.09);
}
