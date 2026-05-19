// Pixel Plumber - backdrop and pipe tile rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#16323f');
  g.addColorStop(1, '#070d12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// Draw a pipe piece in cell (x, y, cell). `openings` is a list of n/e/s/w.
// flooded -> water blue, else dry steel.
function drawPipe(ctx, x, y, cell, openings, flooded) {
  const cx = x + cell / 2, cy = y + cell / 2;
  const w = Math.max(6, Math.round(cell * 0.34));
  const body = flooded ? '#4fb8e8' : '#8893a4';
  const shine = flooded ? '#9fe0f4' : '#aeb8c8';
  ctx.fillStyle = body;
  for (const d of openings) {
    if (d === 'n') ctx.fillRect(cx - w / 2, y, w, cell / 2 + 1);
    else if (d === 's') ctx.fillRect(cx - w / 2, cy, w, cell / 2 + 1);
    else if (d === 'e') ctx.fillRect(cx, cy - w / 2, cell / 2 + 1, w);
    else ctx.fillRect(x, cy - w / 2, cell / 2 + 1, w);
  }
  ctx.fillStyle = body;
  ctx.fillRect(cx - w / 2, cy - w / 2, w, w);
  ctx.fillStyle = shine;
  ctx.fillRect(cx - w / 2 + 1, cy - w / 2 + 1, w - 2, 2);
}

// Source tap — a green block with an arrow pointing the way water leaves.
function drawSource(ctx, x, y, cell, dir) {
  ctx.fillStyle = '#5fc06e';
  ctx.fillRect(x + 3, y + 3, cell - 6, cell - 6);
  ctx.fillStyle = '#0d2a14';
  const cx = x + cell / 2, cy = y + cell / 2, s = cell * 0.2;
  ctx.beginPath();
  if (dir === 'n') { ctx.moveTo(cx, cy - s); ctx.lineTo(cx + s, cy + s); ctx.lineTo(cx - s, cy + s); }
  else if (dir === 's') { ctx.moveTo(cx, cy + s); ctx.lineTo(cx + s, cy - s); ctx.lineTo(cx - s, cy - s); }
  else if (dir === 'e') { ctx.moveTo(cx + s, cy); ctx.lineTo(cx - s, cy + s); ctx.lineTo(cx - s, cy - s); }
  else { ctx.moveTo(cx - s, cy); ctx.lineTo(cx + s, cy + s); ctx.lineTo(cx + s, cy - s); }
  ctx.closePath();
  ctx.fill();
}

// Drain target — a gold ring.
function drawGoal(ctx, x, y, cell, flooded) {
  ctx.strokeStyle = flooded ? '#4fb8e8' : '#f2cf3f';
  ctx.lineWidth = Math.max(3, cell * 0.13);
  ctx.beginPath();
  ctx.arc(x + cell / 2, y + cell / 2, cell * 0.3, 0, Math.PI * 2);
  ctx.stroke();
  ctx.fillStyle = flooded ? '#4fb8e8' : '#f2cf3f';
  ctx.fillRect(x + cell / 2 - 3, y + cell / 2 - 3, 6, 6);
}
