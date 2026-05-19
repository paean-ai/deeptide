// Pixel Data Serpent - pixel art for the grid, serpent, food, firewalls, portals.

function dsShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

function drawGrid(ctx) {
  ctx.fillStyle = '#0a0f18';
  ctx.fillRect(0, 0, CW, CW);
  ctx.fillStyle = '#101826';
  for (let i = 0; i < GRID; i++) {
    for (let j = 0; j < GRID; j++) {
      if ((i + j) % 2 === 0) ctx.fillRect(i * CELL, j * CELL, CELL, CELL);
    }
  }
  ctx.strokeStyle = 'rgba(95,217,192,0.06)';
  ctx.lineWidth = 1;
  for (let i = 1; i < GRID; i++) {
    ctx.beginPath(); ctx.moveTo(i * CELL, 0); ctx.lineTo(i * CELL, CW); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(0, i * CELL); ctx.lineTo(CW, i * CELL); ctx.stroke();
  }
}

// One serpent segment. ratio 0 (head) .. 1 (tail) tints the body.
function drawSegment(ctx, x, y, ratio, isHead, dir, t) {
  const px = x * CELL, py = y * CELL;
  const base = '#3fd9b0';
  const col = dsShade(base, Math.round(-ratio * 70));
  const inset = isHead ? 1 : 2;
  ctx.fillStyle = '#06100e';
  ctx.fillRect(px + 1, py + 1, CELL - 2, CELL - 2);
  ctx.fillStyle = col;
  ctx.fillRect(px + inset, py + inset, CELL - inset * 2, CELL - inset * 2);
  ctx.fillStyle = dsShade(col, 40);
  ctx.fillRect(px + inset, py + inset, CELL - inset * 2, 3);
  // data core
  ctx.fillStyle = isHead ? '#eafff9' : dsShade(base, 60 - ratio * 80);
  const c = CELL / 2;
  ctx.fillRect(px + c - 3, py + c - 3, 6, 6);
  if (isHead) {
    // eyes oriented to travel direction
    ctx.fillStyle = '#0a0f18';
    const ex = dir.x, ey = dir.y;
    const e1x = px + c + ex * 4 - ey * 5, e1y = py + c + ey * 4 - ex * 5;
    const e2x = px + c + ex * 4 + ey * 5, e2y = py + c + ey * 4 + ex * 5;
    ctx.fillRect(e1x - 2, e1y - 2, 4, 4);
    ctx.fillRect(e2x - 2, e2y - 2, 4, 4);
    ctx.fillStyle = '#5fd9ff';
    ctx.fillRect(e1x - 1, e1y - 1, 2, 2);
    ctx.fillRect(e2x - 1, e2y - 1, 2, 2);
  }
}

function drawFood(ctx, x, y, kind, t) {
  const px = x * CELL + CELL / 2, py = y * CELL + CELL / 2;
  const f = FOOD[kind];
  const pulse = (Math.sin(t * 5) + 1) * 0.5;
  ctx.globalAlpha = 0.3 + pulse * 0.35;
  ctx.fillStyle = f.color;
  ctx.fillRect(px - 9, py - 9, 18, 18);
  ctx.globalAlpha = 1;
  ctx.fillStyle = '#06100e';
  ctx.fillRect(px - 7, py - 7, 14, 14);
  ctx.fillStyle = f.color;
  ctx.fillRect(px - 5, py - 5, 10, 10);
  ctx.fillStyle = dsShade(f.color, 70);
  ctx.fillRect(px - 5, py - 5, 10, 3);
  ctx.fillStyle = '#06100e';
  if (kind === 'golden') { ctx.fillRect(px - 1, py - 4, 2, 8); ctx.fillRect(px - 4, py - 1, 8, 2); }
  else if (kind === 'shrink') { ctx.fillRect(px - 4, py - 1, 8, 2); }
  else if (kind === 'slow') { ctx.fillRect(px - 1, py - 4, 2, 5); ctx.fillRect(px - 1, py - 1, 4, 2); }
}

function drawFirewall(ctx, x, y, t) {
  const px = x * CELL, py = y * CELL;
  ctx.fillStyle = '#3a1420';
  ctx.fillRect(px + 1, py + 1, CELL - 2, CELL - 2);
  ctx.fillStyle = '#e0463f';
  ctx.fillRect(px + 3, py + 3, CELL - 6, CELL - 6);
  ctx.fillStyle = '#ff8a6a';
  const flick = (Math.floor(t * 8 + x + y) % 2) ? 3 : 5;
  ctx.fillRect(px + 3, py + 3, CELL - 6, flick);
  ctx.fillStyle = '#7a1f1f';
  ctx.fillRect(px + CELL / 2 - 2, py + 5, 4, CELL - 10);
}

function drawPortal(ctx, x, y, t, which) {
  const px = x * CELL + CELL / 2, py = y * CELL + CELL / 2;
  const col = which === 0 ? '#7a7aff' : '#ff7ad0';
  for (let r = 9; r > 0; r -= 3) {
    ctx.globalAlpha = 0.4 + ((Math.sin(t * 4 + r) + 1) * 0.3);
    ctx.fillStyle = r % 6 === 0 ? col : dsShade(col, 50);
    ctx.fillRect(px - r, py - r, r * 2, r * 2);
  }
  ctx.globalAlpha = 1;
  ctx.fillStyle = '#06100e';
  ctx.fillRect(px - 2, py - 2, 4, 4);
}
