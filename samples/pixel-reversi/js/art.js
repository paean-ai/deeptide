// Pixel Reversi - backdrop and board / disc rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#173a28');
  g.addColorStop(1, '#06100b');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// The green 8x8 felt with grid lines and the four classic guide dots.
function drawBoard(ctx, gx, gy, cell) {
  const size = cell * N;
  ctx.fillStyle = '#2a7a4a';
  ctx.fillRect(gx, gy, size, size);
  ctx.fillStyle = '#236a40';
  ctx.fillRect(gx, gy, size, 4);
  ctx.strokeStyle = '#1c5234';
  ctx.lineWidth = 1;
  for (let i = 1; i < N; i++) {
    ctx.beginPath();
    ctx.moveTo(gx + i * cell, gy); ctx.lineTo(gx + i * cell, gy + size);
    ctx.moveTo(gx, gy + i * cell); ctx.lineTo(gx + size, gy + i * cell);
    ctx.stroke();
  }
  ctx.fillStyle = '#1c5234';
  for (const [r, c] of [[2, 2], [2, 6], [6, 2], [6, 6]]) {
    ctx.beginPath();
    ctx.arc(gx + c * cell, gy + r * cell, 3, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawDisc(ctx, cx, cy, r, player) {
  ctx.fillStyle = 'rgba(0,0,0,0.35)';
  ctx.beginPath();
  ctx.arc(cx, cy + 2, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = player === DARK ? '#1a1f24' : '#f3f0e6';
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = player === DARK ? 'rgba(255,255,255,0.16)' : 'rgba(255,255,255,0.55)';
  ctx.beginPath();
  ctx.arc(cx - r * 0.3, cy - r * 0.32, r * 0.36, 0, Math.PI * 2);
  ctx.fill();
}

// a small dot marking a legal move for the player
function drawHint(ctx, cx, cy) {
  ctx.fillStyle = 'rgba(255,255,255,0.4)';
  ctx.beginPath();
  ctx.arc(cx, cy, 5, 0, Math.PI * 2);
  ctx.fill();
}
