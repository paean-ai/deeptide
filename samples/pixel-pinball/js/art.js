// Pixel Pinball - table, bumper, target, flipper and ball rendering.

function drawTable(ctx, time) {
  // playfield
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1a2540');
  g.addColorStop(1, '#0c1226');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // subtle pixel grid
  ctx.fillStyle = 'rgba(120,150,210,0.05)';
  for (let y = 16; y < VH; y += 16) {
    for (let x = 16; x < VW; x += 16) {
      if ((x / 16 + y / 16) % 2 === 0) ctx.fillRect(x, y, 16, 16);
    }
  }
  // glowing arc lanes at the top
  ctx.strokeStyle = 'rgba(120,200,255,0.16)';
  ctx.lineWidth = 5;
  for (let i = 0; i < 3; i++) {
    ctx.beginPath();
    ctx.arc(160, 60, 56 + i * 16, Math.PI * 1.12, Math.PI * 1.88);
    ctx.stroke();
  }
}

function drawWalls(ctx, walls) {
  ctx.lineCap = 'round';
  for (const w of walls) {
    ctx.strokeStyle = '#2c3a63';
    ctx.lineWidth = 9;
    ctx.beginPath(); ctx.moveTo(w[0], w[1]); ctx.lineTo(w[2], w[3]); ctx.stroke();
    ctx.strokeStyle = '#6f8fd6';
    ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(w[0], w[1]); ctx.lineTo(w[2], w[3]); ctx.stroke();
  }
}

function drawBumper(ctx, bm, flash) {
  const r = bm.r;
  ctx.fillStyle = '#10193a';
  ctx.beginPath(); ctx.arc(bm.x, bm.y, r + 3, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = flash > 0 ? '#fff4c2' : '#ff9d3d';
  ctx.beginPath(); ctx.arc(bm.x, bm.y, r * (1 + flash * 0.12), 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = flash > 0 ? '#ffffff' : '#ffd089';
  ctx.beginPath(); ctx.arc(bm.x, bm.y, r * 0.62, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#1a1330';
  ctx.beginPath(); ctx.arc(bm.x, bm.y, r * 0.3, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = flash > 0 ? '#ffffff' : 'rgba(255,220,150,0.5)';
  ctx.lineWidth = 2;
  ctx.beginPath(); ctx.arc(bm.x, bm.y, r + 1, 0, Math.PI * 2); ctx.stroke();
}

function drawSling(ctx, s, flash) {
  ctx.fillStyle = '#10193a';
  ctx.beginPath(); ctx.arc(s.x, s.y, s.r + 3, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = flash > 0 ? '#ffffff' : '#5be0a0';
  ctx.beginPath(); ctx.arc(s.x, s.y, s.r, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#0d3a2a';
  ctx.beginPath(); ctx.arc(s.x, s.y, s.r * 0.4, 0, Math.PI * 2); ctx.fill();
}

function drawTarget(ctx, rect, up) {
  const [x, y, w, h] = rect;
  if (up) {
    ctx.fillStyle = '#0c1530';
    ctx.fillRect(x - 1, y - 1, w + 2, h + 2);
    ctx.fillStyle = '#f2d24a';
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = '#fff6c4';
    ctx.fillRect(x, y, w, 3);
    ctx.fillStyle = '#9c8418';
    ctx.fillRect(x, y + h - 3, w, 3);
  } else {
    ctx.fillStyle = 'rgba(60,72,110,0.5)';
    ctx.fillRect(x + 3, y + h - 4, w - 6, 4);
  }
}

function drawFlipper(ctx, fl, angle) {
  const tx = fl.px + Math.cos(angle) * fl.len;
  const ty = fl.py + Math.sin(angle) * fl.len;
  ctx.lineCap = 'round';
  ctx.strokeStyle = '#243056';
  ctx.lineWidth = FLIP_THICK * 2 + 4;
  ctx.beginPath(); ctx.moveTo(fl.px, fl.py); ctx.lineTo(tx, ty); ctx.stroke();
  ctx.strokeStyle = '#e8554f';
  ctx.lineWidth = FLIP_THICK * 2;
  ctx.beginPath(); ctx.moveTo(fl.px, fl.py); ctx.lineTo(tx, ty); ctx.stroke();
  ctx.strokeStyle = '#ff938d';
  ctx.lineWidth = 3;
  ctx.beginPath(); ctx.moveTo(fl.px, fl.py); ctx.lineTo(tx, ty); ctx.stroke();
  // pivot cap
  ctx.fillStyle = '#cdd8f5';
  ctx.beginPath(); ctx.arc(fl.px, fl.py, 5, 0, Math.PI * 2); ctx.fill();
}

function drawBall(ctx, x, y) {
  ctx.fillStyle = 'rgba(0,0,0,0.35)';
  ctx.beginPath(); ctx.ellipse(x + 2, y + 3, BALL_R, BALL_R * 0.7, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#c6d0e4';
  ctx.beginPath(); ctx.arc(x, y, BALL_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#eef3ff';
  ctx.beginPath(); ctx.arc(x - 1.5, y - 1.5, BALL_R * 0.6, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(x - 3, y - 4, 2, 2);
}

function drawSparks(ctx, sparks) {
  for (const s of sparks) {
    ctx.globalAlpha = Math.max(0, s.life);
    ctx.fillStyle = s.color;
    ctx.fillRect(s.x - 2, s.y - 2, 4, 4);
  }
  ctx.globalAlpha = 1;
}
