// Pixel Peg Pop - backdrop and sprite drawing.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#241a3e');
  g.addColorStop(1, '#090714');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A peg. blue = normal, orange = target; lit pegs glow until the turn ends.
function drawPeg(ctx, peg) {
  if (peg.cleared) return;
  const target = peg.target;
  let base = target ? '#ff9e42' : '#4a8be0';
  if (peg.lit) base = target ? '#ffd27a' : '#9fc8ff';
  ctx.fillStyle = base;
  ctx.beginPath();
  ctx.arc(peg.x, peg.y, PEG_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = peg.lit ? '#ffffff' : (target ? '#ffd9a8' : '#bcd6ff');
  ctx.fillRect(Math.round(peg.x - PEG_R * 0.45), Math.round(peg.y - PEG_R * 0.55), 4, 4);
  if (peg.lit) {
    ctx.strokeStyle = 'rgba(255,255,255,0.7)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(peg.x, peg.y, PEG_R + 2, 0, Math.PI * 2);
    ctx.stroke();
  }
}

function drawBall(ctx, b) {
  ctx.fillStyle = '#f3f3fa';
  ctx.beginPath();
  ctx.arc(b.x, b.y, BALL_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#9fa6c8';
  ctx.fillRect(Math.round(b.x - 1), Math.round(b.y), 3, 3);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(Math.round(b.x - 3), Math.round(b.y - 3), 2, 2);
}

// The launcher at the top, barrel rotated to the current aim angle.
function drawLauncher(ctx, angle) {
  ctx.save();
  ctx.translate(LAUNCH_X, LAUNCH_Y);
  ctx.rotate(angle);
  ctx.fillStyle = '#c98b3a';
  ctx.fillRect(-5, 0, 10, 22);
  ctx.fillStyle = '#1a1228';
  ctx.fillRect(-4, 16, 8, 6);
  ctx.restore();
  ctx.fillStyle = '#e0a64a';
  ctx.beginPath();
  ctx.arc(LAUNCH_X, LAUNCH_Y, 9, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#fff0d0';
  ctx.fillRect(LAUNCH_X - 3, LAUNCH_Y - 3, 4, 4);
}

// The roaming catcher bucket along the bottom.
function drawBucket(ctx, x, y) {
  ctx.fillStyle = '#6fd0d0';
  ctx.fillRect(x - 22, y, 6, 16);
  ctx.fillRect(x + 16, y, 6, 16);
  ctx.fillStyle = 'rgba(111,208,208,0.4)';
  ctx.fillRect(x - 16, y + 6, 32, 10);
}

function drawAimDots(ctx, angle) {
  let x = LAUNCH_X + Math.cos(angle) * 14;
  let y = LAUNCH_Y + Math.sin(angle) * 14;
  let vx = Math.cos(angle) * LAUNCH_SPEED;
  let vy = Math.sin(angle) * LAUNCH_SPEED;
  ctx.fillStyle = 'rgba(255,255,255,0.6)';
  for (let i = 0; i < 22; i++) {
    for (let s = 0; s < 3; s++) { vy += GRAVITY * 0.014; x += vx * 0.014; y += vy * 0.014; }
    if (y > VH || x < 0 || x > VW) break;
    if (i % 2 === 0) ctx.fillRect(x - 1.5, y - 1.5, 3, 3);
  }
}
