// Pixel Burrow - rendering.

const COL = {
  dirt: '#7a5230', dirtDark: '#5c3d22', hole: '#241608', holeRim: '#3e2a14',
  gopher: '#c98a4a', gopherDark: '#9a6634', golden: '#ffd23e', goldenDark: '#c89a17',
  bomb: '#3a3f4c', bombDark: '#22252e', fuse: '#ff7a3a',
  belly: '#f0dcc0', eye: '#1a1208',
};

function drawBackground(ctx, flash) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#3a6e3c');
  g.addColorStop(0.5, '#2c5230');
  g.addColorStop(1, '#1a3420');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.04)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
  if (flash > 0) {
    ctx.fillStyle = `rgba(255,80,80,${flash * 0.7})`;
    ctx.fillRect(0, 0, VW, VH);
  }
}

function burrowGeom() {
  const cell = 100, gx = (VW - cell * 3) / 2, gy = 132;
  return { cell, gx, gy };
}
function burrowRect(i) {
  const g = burrowGeom();
  return { x: g.gx + (i % 3) * g.cell, y: g.gy + ((i / 3) | 0) * g.cell, s: g.cell };
}

function drawBurrows(ctx, state) {
  for (let i = 0; i < BURROWS; i++) {
    const r = burrowRect(i);
    const cx = r.x + r.s / 2, cy = r.y + r.s * 0.62;
    const rw = r.s * 0.38, rh = r.s * 0.2;
    // mound
    ctx.fillStyle = COL.dirt;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rw + 9, rh + 7, 0, 0, Math.PI * 2);
    ctx.fill();
    // hole
    ctx.fillStyle = COL.hole;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rw, rh, 0, 0, Math.PI * 2);
    ctx.fill();
    const c = state.burrows[i];
    if (c) drawCritter(ctx, cx, cy, rw, rh, c);
    // rim front
    ctx.strokeStyle = COL.holeRim;
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rw, rh, 0, 0.05 * Math.PI, 0.95 * Math.PI);
    ctx.stroke();
  }
}

function critterReveal(c) {
  const rise = Math.min(1, c.age / 0.13);
  const sink = c.age > c.life - 0.18 ? Math.max(0, (c.life - c.age) / 0.18) : 1;
  return Math.min(rise, sink);
}

function drawCritter(ctx, cx, cy, rw, rh, c) {
  const rev = critterReveal(c);
  if (rev <= 0) return;
  const h = rw * 1.5 * rev;          // how far it rises
  const bodyW = rw * 1.1, bodyH = rw * 1.3;
  const topY = cy - h;
  ctx.save();
  // clip to the hole so it emerges
  ctx.beginPath();
  ctx.rect(cx - rw - 12, cy - rw * 2, (rw + 12) * 2, h + rh);
  ctx.clip();
  if (c.type === 'bomb') {
    ctx.fillStyle = COL.bombDark;
    ctx.beginPath();
    ctx.arc(cx, topY + bodyH * 0.5, bodyW * 0.62, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = COL.bomb;
    ctx.beginPath();
    ctx.arc(cx, topY + bodyH * 0.5, bodyW * 0.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = COL.fuse;
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(cx, topY + bodyH * 0.04);
    ctx.lineTo(cx + 7, topY - bodyH * 0.16);
    ctx.stroke();
    ctx.fillStyle = '#ffd23e';
    ctx.beginPath();
    ctx.arc(cx + 7, topY - bodyH * 0.16, 3.5, 0, Math.PI * 2);
    ctx.fill();
  } else {
    const dark = c.type === 'golden' ? COL.goldenDark : COL.gopherDark;
    const main = c.type === 'golden' ? COL.golden : COL.gopher;
    ctx.fillStyle = dark;
    ctx.fillRect(cx - bodyW / 2 - 3, topY + 2, bodyW + 6, bodyH);
    ctx.fillStyle = main;
    ctx.fillRect(cx - bodyW / 2, topY, bodyW, bodyH);
    // ears
    ctx.fillStyle = main;
    ctx.fillRect(cx - bodyW / 2 - 2, topY - 6, 9, 12);
    ctx.fillRect(cx + bodyW / 2 - 7, topY - 6, 9, 12);
    // belly
    ctx.fillStyle = COL.belly;
    ctx.fillRect(cx - bodyW * 0.26, topY + bodyH * 0.45, bodyW * 0.52, bodyH * 0.4);
    // eyes
    ctx.fillStyle = COL.eye;
    ctx.fillRect(cx - bodyW * 0.26, topY + bodyH * 0.22, 7, 8);
    ctx.fillRect(cx + bodyW * 0.26 - 7, topY + bodyH * 0.22, 7, 8);
    // nose
    ctx.fillStyle = '#d0506a';
    ctx.fillRect(cx - 3, topY + bodyH * 0.4, 6, 5);
  }
  ctx.restore();
}
