// Pixel Tower Stack - drawing helpers for blocks, debris, sparkles, backdrop.

// A single tower block. hue picks the colour; `lift` tints the moving block.
function drawBlock(ctx, x, y, w, h, hue, moving) {
  const light = moving ? 64 : 56;
  ctx.fillStyle = `hsl(${hue}, 62%, ${light - 22}%)`;
  ctx.fillRect(x | 0, y | 0, Math.ceil(w), h);                 // body / shade base
  ctx.fillStyle = `hsl(${hue}, 60%, ${light}%)`;
  ctx.fillRect(x | 0, y | 0, Math.ceil(w), h - 5);             // main face
  ctx.fillStyle = `hsl(${hue}, 70%, ${light + 16}%)`;
  ctx.fillRect(x | 0, y | 0, Math.ceil(w), 4);                 // top highlight
  ctx.fillStyle = `hsl(${hue}, 55%, ${light - 14}%)`;
  ctx.fillRect((x | 0) + Math.ceil(w) - 4, y | 0, 4, h);       // right edge
  if (moving) {
    ctx.fillStyle = 'rgba(255,255,255,0.14)';
    ctx.fillRect(x | 0, y | 0, Math.ceil(w), h);
  }
}

function drawDebris(ctx, d) {
  ctx.globalAlpha = Math.max(0, Math.min(1, d.life));
  ctx.fillStyle = `hsl(${d.hue}, 60%, 52%)`;
  ctx.fillRect(d.x | 0, d.y | 0, Math.ceil(d.w), Math.ceil(d.h));
  ctx.fillStyle = `hsl(${d.hue}, 70%, 66%)`;
  ctx.fillRect(d.x | 0, d.y | 0, Math.ceil(d.w), 3);
  ctx.globalAlpha = 1;
}

function drawSparkle(ctx, s) {
  ctx.globalAlpha = Math.max(0, s.life);
  ctx.strokeStyle = '#fff2b0';
  ctx.lineWidth = 2;
  const r = (1 - s.life) * 26 + 4;
  ctx.strokeRect(s.x - r, s.y - r, r * 2, r * 2);
  ctx.globalAlpha = 1;
}

// Sky backdrop whose tint rises with the tower.
function drawBackdrop(ctx, w, h, height, t) {
  const band = Math.min(1, height / 140);
  const top = `hsl(${(220 + band * 120) % 360}, 42%, ${10 + band * 8}%)`;
  const bot = `hsl(${(248 + band * 90) % 360}, 38%, ${20 + band * 14}%)`;
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, top);
  g.addColorStop(1, bot);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  // drifting stars
  ctx.fillStyle = 'rgba(255,255,255,0.5)';
  for (let i = 0; i < 22; i++) {
    const sx = (i * 71 + 13) % w;
    const sy = (i * 53 + t * 6) % h;
    ctx.fillRect(sx, sy, i % 4 === 0 ? 2 : 1, i % 4 === 0 ? 2 : 1);
  }
}
