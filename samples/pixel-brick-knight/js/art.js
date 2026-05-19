// Pixel Brick Knight - backdrop and sprite drawing.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#2a2140');
  g.addColorStop(1, '#0a0810');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // faint dungeon-brick wall texture
  ctx.fillStyle = 'rgba(255,255,255,0.025)';
  for (let y = 0; y < VH; y += 24) {
    const off = (y / 24) % 2 ? 16 : 0;
    for (let x = -16 + off; x < VW; x += 32) ctx.fillRect(x, y, 30, 22);
  }
}

// A brick — colour by remaining hp, with a chunky bevel.
function drawBrick(ctx, b) {
  const ratio = b.hp / b.maxhp;
  let col;
  if (b.kind === 'boss') col = '#b23a52';
  else col = BRICK_COLORS[Math.min(BRICK_COLORS.length - 1, b.hp - 1)];
  ctx.fillStyle = col;
  ctx.fillRect(b.x, b.y, b.w, b.h);
  ctx.fillStyle = 'rgba(255,255,255,0.28)';
  ctx.fillRect(b.x, b.y, b.w, 3);
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.fillRect(b.x, b.y + b.h - 3, b.w, 3);
  if (b.kind === 'boss') {
    ctx.fillStyle = '#1a0c10';
    ctx.fillRect(b.x + b.w * 0.3 - 4, b.y + b.h * 0.4, 7, 7);
    ctx.fillRect(b.x + b.w * 0.7 - 3, b.y + b.h * 0.4, 7, 7);
    // hp bar
    ctx.fillStyle = '#1a0c10';
    ctx.fillRect(b.x, b.y - 6, b.w, 4);
    ctx.fillStyle = '#e8554f';
    ctx.fillRect(b.x, b.y - 6, b.w * ratio, 4);
  } else if (b.maxhp > 1) {
    ctx.fillStyle = 'rgba(0,0,0,0.3)';
    ctx.font = '900 9px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(String(b.hp), b.x + b.w / 2, b.y + b.h / 2 + 3);
    ctx.textAlign = 'left';
  }
}

function drawPaddle(ctx, x, y, w, h) {
  ctx.fillStyle = '#c89bff';
  ctx.fillRect(x - w / 2, y, w, h);
  ctx.fillStyle = '#e6d2ff';
  ctx.fillRect(x - w / 2, y, w, 3);
  ctx.fillStyle = '#6e4aa0';
  ctx.fillRect(x - w / 2, y + h - 3, w, 3);
}

function drawBall(ctx, b) {
  ctx.fillStyle = '#fff4c2';
  ctx.beginPath();
  ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#f2cf3f';
  ctx.fillRect(Math.round(b.x - 1), Math.round(b.y - 1), 3, 3);
}
