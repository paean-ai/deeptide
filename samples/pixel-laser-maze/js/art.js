// Pixel Laser Maze - backdrop and grid-element rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#241834');
  g.addColorStop(1, '#08060e');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawCell(ctx, x, y, s) {
  ctx.fillStyle = '#16101f';
  ctx.fillRect(x, y, s, s);
  ctx.strokeStyle = '#2c2138';
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, s - 1, s - 1);
}

function drawWall(ctx, x, y, s) {
  ctx.fillStyle = '#4a3f56';
  ctx.fillRect(x + 2, y + 2, s - 4, s - 4);
  ctx.fillStyle = '#5e5168';
  ctx.fillRect(x + 2, y + 2, s - 4, 3);
  ctx.fillStyle = '#332b3e';
  ctx.fillRect(x + 2, y + s - 5, s - 4, 3);
}

function drawEmitter(ctx, x, y, s, dir) {
  const cx = x + s / 2, cy = y + s / 2;
  ctx.fillStyle = '#2a3550';
  ctx.fillRect(x + 4, y + 4, s - 8, s - 8);
  ctx.fillStyle = '#ff5a78';
  const m = s * 0.18;
  ctx.beginPath();
  if (dir[0] === 1) { ctx.moveTo(cx + m, cy); ctx.lineTo(cx - m, cy - m); ctx.lineTo(cx - m, cy + m); }
  else if (dir[0] === -1) { ctx.moveTo(cx - m, cy); ctx.lineTo(cx + m, cy - m); ctx.lineTo(cx + m, cy + m); }
  else if (dir[1] === 1) { ctx.moveTo(cx, cy + m); ctx.lineTo(cx - m, cy - m); ctx.lineTo(cx + m, cy - m); }
  else { ctx.moveTo(cx, cy - m); ctx.lineTo(cx - m, cy + m); ctx.lineTo(cx + m, cy + m); }
  ctx.closePath();
  ctx.fill();
}

function drawTarget(ctx, x, y, s, lit) {
  const cx = x + s / 2, cy = y + s / 2, r = s * 0.3;
  if (lit) {
    ctx.fillStyle = 'rgba(95,214,192,0.32)';
    ctx.beginPath();
    ctx.arc(cx, cy, r * 1.7, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = lit ? '#5fd6c0' : '#3a6a62';
  ctx.beginPath();
  ctx.moveTo(cx, cy - r); ctx.lineTo(cx + r, cy);
  ctx.lineTo(cx, cy + r); ctx.lineTo(cx - r, cy);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = lit ? '#d6fff5' : '#5a8a82';
  ctx.fillRect(cx - 2, cy - 2, 4, 4);
}

function drawMirror(ctx, x, y, s, type) {
  ctx.strokeStyle = '#cfe0ff';
  ctx.lineWidth = Math.max(3, s * 0.14);
  ctx.lineCap = 'round';
  const m = s * 0.24;
  ctx.beginPath();
  if (type === '/') { ctx.moveTo(x + s - m, y + m); ctx.lineTo(x + m, y + s - m); }
  else { ctx.moveTo(x + m, y + m); ctx.lineTo(x + s - m, y + s - m); }
  ctx.stroke();
  ctx.strokeStyle = 'rgba(95,214,192,0.6)';
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

// The laser beam as a glowing polyline through cell-centre points.
function drawBeam(ctx, points) {
  if (points.length < 2) return;
  ctx.strokeStyle = 'rgba(255,90,120,0.3)';
  ctx.lineWidth = 7;
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length; i++) ctx.lineTo(points[i].x, points[i].y);
  ctx.stroke();
  ctx.strokeStyle = '#ff8fa3';
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length; i++) ctx.lineTo(points[i].x, points[i].y);
  ctx.stroke();
}
