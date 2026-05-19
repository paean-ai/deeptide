// Pixel Flow - backdrop and grid / pipe rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#172238');
  g.addColorStop(1, '#070a12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// The empty grid: a recessed board with thin cell lines.
function drawGrid(ctx, gx, gy, n, cell) {
  ctx.fillStyle = '#10182c';
  ctx.fillRect(gx - 2, gy - 2, n * cell + 4, n * cell + 4);
  ctx.strokeStyle = '#243150';
  ctx.lineWidth = 1;
  for (let i = 0; i <= n; i++) {
    ctx.beginPath();
    ctx.moveTo(gx + i * cell, gy);
    ctx.lineTo(gx + i * cell, gy + n * cell);
    ctx.moveTo(gx, gy + i * cell);
    ctx.lineTo(gx + n * cell, gy + i * cell);
    ctx.stroke();
  }
}

// A colour's pipe — a thick rounded ribbon through the path's cell centres.
function drawPipe(ctx, pts, color, cell, active) {
  if (pts.length < 1) return;
  ctx.strokeStyle = color;
  ctx.lineWidth = cell * (active ? 0.46 : 0.4);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  if (pts.length >= 2) {
    ctx.beginPath();
    ctx.moveTo(pts[0].x, pts[0].y);
    for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y);
    ctx.stroke();
  }
  // a soft square nub at the live head so a 1-cell path still reads
  const h = pts[pts.length - 1];
  ctx.fillStyle = color;
  const s = cell * 0.34;
  ctx.fillRect(h.x - s / 2, h.y - s / 2, s, s);
}

// An endpoint dot.
function drawEndpoint(ctx, x, y, color, cell, done) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(x, y, cell * 0.34, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = done ? '#ffffff' : 'rgba(255,255,255,0.55)';
  ctx.beginPath();
  ctx.arc(x, y, cell * 0.13, 0, Math.PI * 2);
  ctx.fill();
}
