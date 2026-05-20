// Pixel Fruit Slash - backdrop and pixel sprite drawing.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#2a1830');
  g.addColorStop(1, '#0e0810');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // soft spotlight planks at the base
  ctx.fillStyle = 'rgba(255, 143, 74, 0.06)';
  ctx.beginPath();
  ctx.moveTo(VW / 2, 40);
  ctx.lineTo(VW + 40, VH);
  ctx.lineTo(-40, VH);
  ctx.closePath();
  ctx.fill();
}

// A whole fruit: body circle, shine, leaf nub.
function drawFruit(ctx, o) {
  const f = o.def;
  // Rare golden fruit gets a pulsing halo so the player notices it.
  if (o.kind === 'gold') {
    const pulse = 0.5 + 0.5 * Math.sin((o.t || 0) * 9);
    ctx.fillStyle = 'rgba(255, 224, 120, ' + (0.22 + pulse * 0.2).toFixed(3) + ')';
    ctx.beginPath();
    ctx.arc(o.x, o.y, f.r + 6 + pulse * 2, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = f.color;
  ctx.beginPath();
  ctx.arc(o.x, o.y, f.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = f.inner;
  ctx.fillRect(Math.round(o.x - f.r * 0.5), Math.round(o.y - f.r * 0.6),
               Math.max(3, f.r * 0.34), Math.max(3, f.r * 0.34));
  ctx.fillStyle = '#3f7a3a';
  ctx.fillRect(Math.round(o.x - 3), Math.round(o.y - f.r - 4), 6, 6);
}

// A spinning fuse bomb.
function drawBomb(ctx, o, t) {
  ctx.fillStyle = BOMB.color;
  ctx.beginPath();
  ctx.arc(o.x, o.y, BOMB.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#4a4a56';
  ctx.fillRect(Math.round(o.x - BOMB.r * 0.45), Math.round(o.y - BOMB.r * 0.5), 6, 6);
  // fuse + flickering spark
  ctx.fillStyle = '#6e5a3a';
  ctx.fillRect(Math.round(o.x - 2), Math.round(o.y - BOMB.r - 7), 4, 8);
  ctx.fillStyle = (t * 18 | 0) % 2 ? '#ffd23f' : BOMB.spark;
  ctx.fillRect(Math.round(o.x - 3), Math.round(o.y - BOMB.r - 11), 6, 6);
}

// One sliced half (a filled half-disc) flung from a cut fruit.
function drawHalf(ctx, h) {
  ctx.save();
  ctx.translate(h.x, h.y);
  ctx.rotate(h.rot);
  ctx.globalAlpha = Math.min(1, h.life * 1.8);
  const a0 = h.side === 'L' ? Math.PI / 2 : -Math.PI / 2;
  const a1 = h.side === 'L' ? Math.PI * 1.5 : Math.PI / 2;
  ctx.fillStyle = h.color;
  ctx.beginPath();
  ctx.arc(0, 0, h.r, a0, a1);
  ctx.closePath();
  ctx.fill();
  // juicy cut face along the flat edge
  ctx.fillStyle = h.inner;
  ctx.fillRect(h.side === 'L' ? 0 : -3, -h.r + 2, 3, h.r * 2 - 4);
  ctx.globalAlpha = 1;
  ctx.restore();
}

// The blade trail — a bright tapering ribbon following recent swipe points.
function drawBlade(ctx, points) {
  if (points.length < 2) return;
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1], b = points[i];
    const w = 1 + 7 * (i / points.length) * a.life;
    ctx.strokeStyle = 'rgba(120, 230, 255, ' + (0.5 * a.life) + ')';
    ctx.lineWidth = w + 4;
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
    ctx.stroke();
    ctx.strokeStyle = 'rgba(255, 255, 255, ' + a.life + ')';
    ctx.lineWidth = w;
    ctx.beginPath();
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
    ctx.stroke();
  }
}
