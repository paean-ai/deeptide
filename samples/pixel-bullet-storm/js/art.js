// Pixel Bullet Storm - starfield + pixel sprite drawing.

// Player ship bitmap: 1 = hull, 2 = cockpit, 3 = engine flame.
const SHIP = [
  '....1....',
  '...111...',
  '...121...',
  '..11111..',
  '.1111111.',
  '111111111',
  '11.131.11',
  '....3....',
];
const SHIP_PAL = { 1: '#9fd6ff', 2: '#ffffff', 3: '#ff9b3e' };

function makeStars(n) {
  const stars = [];
  for (let i = 0; i < n; i++) {
    stars.push({
      x: Math.random() * VW,
      y: Math.random() * VH,
      sp: 18 + Math.random() * 46,
      sz: Math.random() < 0.7 ? 1 : 2,
    });
  }
  return stars;
}

function drawBackground(ctx, w, h, stars) {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, '#0c0c22');
  g.addColorStop(1, '#04040c');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  if (stars) {
    ctx.fillStyle = '#3a3a64';
    for (const s of stars) ctx.fillRect(s.x | 0, s.y | 0, s.sz, s.sz);
  }
}

// Draw a bitmap centred at (cx, cy); px is the size of one source pixel.
function drawBitmap(ctx, rows, pal, cx, cy, px) {
  const ox = Math.round(cx - rows[0].length * px / 2);
  const oy = Math.round(cy - rows.length * px / 2);
  for (let y = 0; y < rows.length; y++) {
    for (let x = 0; x < rows[y].length; x++) {
      const c = rows[y][x];
      if (c === '.' || c === '0') continue;
      ctx.fillStyle = pal[c];
      ctx.fillRect(ox + x * px, oy + y * px, px, px);
    }
  }
}

function drawShip(ctx, x, y, flamePhase) {
  drawBitmap(ctx, SHIP, SHIP_PAL, x, y, 2);
  // flickering engine trail
  const len = 5 + (flamePhase ? 4 : 0);
  ctx.fillStyle = 'rgba(255, 155, 62, 0.7)';
  ctx.fillRect(Math.round(x - 2), Math.round(y + 8), 4, len);
}

// The hitbox dot — the player's true collision point.
function drawCore(ctx, x, y, invuln) {
  if (invuln) {
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.6)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(x, y, 9, 0, Math.PI * 2);
    ctx.stroke();
  }
  ctx.fillStyle = '#fff';
  ctx.fillRect(Math.round(x - 2), Math.round(y - 2), 4, 4);
  ctx.fillStyle = '#ff5fa8';
  ctx.fillRect(Math.round(x - 1), Math.round(y - 1), 2, 2);
}

function drawEnemy(ctx, e) {
  const { x, y } = e, r = e.def.r, col = e.def.color;
  const hit = e.hitFlash > 0;
  ctx.fillStyle = hit ? '#ffffff' : col;
  if (e.type === 'drone') {
    ctx.beginPath();
    ctx.moveTo(x, y - r); ctx.lineTo(x + r, y);
    ctx.lineTo(x, y + r); ctx.lineTo(x - r, y);
    ctx.closePath(); ctx.fill();
  } else if (e.type === 'turret') {
    ctx.fillRect(x - r, y - r, r * 2, r * 2);
    ctx.fillStyle = hit ? '#fff' : '#1a1a32';
    ctx.fillRect(x - 3, y + r - 2, 6, 7);
  } else if (e.type === 'weaver') {
    ctx.beginPath();
    ctx.moveTo(x - r, y - r * 0.5); ctx.lineTo(x, y - r * 0.1);
    ctx.lineTo(x + r, y - r * 0.5); ctx.lineTo(x + r * 0.5, y + r);
    ctx.lineTo(x - r * 0.5, y + r);
    ctx.closePath(); ctx.fill();
  } else { // spinner
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = hit ? '#fff' : '#1a1a32';
    ctx.lineWidth = 3;
    for (let i = 0; i < 3; i++) {
      const a = e.spin + i * Math.PI * 2 / 3;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
      ctx.stroke();
    }
  }
  if (!hit) {
    ctx.fillStyle = '#1a1a32';
    ctx.fillRect(x - 4, y - 3, 3, 3);
    ctx.fillRect(x + 1, y - 3, 3, 3);
  }
}

function drawBoss(ctx, b) {
  const { x, y } = b, r = b.def.r;
  const hit = b.hitFlash > 0;
  ctx.fillStyle = hit ? '#ffffff' : '#ff6b6b';
  ctx.fillRect(x - r, y - r * 0.7, r * 2, r * 1.4);
  ctx.beginPath();
  ctx.arc(x, y, r * 0.85, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = hit ? '#fff' : '#7a1f2a';
  ctx.fillRect(x - r * 0.9, y + r * 0.2, r * 1.8, 6);
  // glaring eyes
  ctx.fillStyle = hit ? '#1a1a32' : '#ffd23f';
  ctx.fillRect(x - 12, y - 6, 7, 7);
  ctx.fillRect(x + 5, y - 6, 7, 7);
}

function drawBullet(ctx, b) {
  ctx.fillStyle = b.color;
  ctx.beginPath();
  ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(Math.round(b.x - 1), Math.round(b.y - 1), 2, 2);
}

function drawShot(ctx, s) {
  ctx.fillStyle = '#bff4ff';
  ctx.fillRect(Math.round(s.x - 1.5), Math.round(s.y - 5), 3, 9);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(Math.round(s.x - 1.5), Math.round(s.y - 5), 3, 3);
}
