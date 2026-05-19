// Pixel Bubble Pop - bubble, launcher and aim-line rendering.

function bpShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// A bubble centred at (cx, cy). `scale` for pop / spawn animation.
function drawBubble(ctx, cx, cy, color, scale) {
  const r = (BUB / 2 - 1) * (scale == null ? 1 : scale);
  if (r <= 0) return;
  ctx.fillStyle = bpShade(color, -70);
  ctx.fillRect(cx - r, cy - r, r * 2, r * 2);
  ctx.fillStyle = color;
  ctx.fillRect(cx - r + 2, cy - r + 2, r * 2 - 4, r * 2 - 4);
  ctx.fillStyle = bpShade(color, 56);
  ctx.fillRect(cx - r + 2, cy - r + 2, r * 2 - 4, 4);
  ctx.fillStyle = bpShade(color, -36);
  ctx.fillRect(cx - r + 2, cy + r - 6, r * 2 - 4, 4);
  // glint
  ctx.fillStyle = bpShade(color, 90);
  ctx.fillRect(cx - r + 5, cy - r + 5, Math.max(2, r * 0.35), Math.max(2, r * 0.35));
}

function drawLauncher(ctx, cx, cy, angle, nextColor) {
  // barrel
  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate(angle);
  ctx.fillStyle = '#3a4055';
  ctx.fillRect(-7, -34, 14, 34);
  ctx.fillStyle = '#565d77';
  ctx.fillRect(-7, -34, 4, 34);
  ctx.restore();
  // base
  ctx.fillStyle = '#2a2f44';
  ctx.fillRect(cx - 22, cy - 8, 44, 22);
  ctx.fillStyle = '#3c4360';
  ctx.fillRect(cx - 22, cy - 8, 44, 5);
  // loaded bubble
  drawBubble(ctx, cx, cy, nextColor, 1);
}

function drawAim(ctx, cx, cy, angle) {
  // dotted aim line that bounces off the side walls
  let x = cx, y = cy;
  let vx = Math.sin(angle), vy = -Math.cos(angle);
  ctx.fillStyle = 'rgba(255,255,255,0.5)';
  for (let i = 0; i < 60; i++) {
    x += vx * 14; y += vy * 14;
    if (x < BUB / 2) { x = BUB / 2; vx = -vx; }
    if (x > VW - BUB / 2) { x = VW - BUB / 2; vx = -vx; }
    if (y < BUB / 2) break;
    if (i % 2 === 0) ctx.fillRect(x - 2, y - 2, 4, 4);
  }
}
