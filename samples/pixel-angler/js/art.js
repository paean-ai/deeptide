// Pixel Angler - scene and sprite drawing.

const WATER_TOP = 150;

function drawScene(ctx, waters, lineX, lineY) {
  // sky
  const sky = ctx.createLinearGradient(0, 0, 0, WATER_TOP);
  sky.addColorStop(0, '#3a5e84');
  sky.addColorStop(1, '#6f9ab0');
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, VW, WATER_TOP);
  // water — darker the deeper the waters tier
  const tints = [['#3b78a0', '#1d4a68'], ['#356f9a', '#173c5a'], ['#2c5e8c', '#0f2c46']];
  const w = ctx.createLinearGradient(0, WATER_TOP, 0, VH);
  w.addColorStop(0, tints[waters][0]);
  w.addColorStop(1, tints[waters][1]);
  ctx.fillStyle = w;
  ctx.fillRect(0, WATER_TOP, VW, VH - WATER_TOP);
  // surface band
  ctx.fillStyle = 'rgba(255,255,255,0.12)';
  ctx.fillRect(0, WATER_TOP, VW, 4);
  // dock + angler at the top-left
  ctx.fillStyle = '#6e4a2a';
  ctx.fillRect(20, WATER_TOP - 12, 96, 12);
  drawAngler(ctx, 54, WATER_TOP - 12);
  // fishing line
  if (lineY != null) {
    ctx.strokeStyle = 'rgba(235,242,248,0.55)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(96, WATER_TOP - 30);
    ctx.lineTo(lineX, lineY);
    ctx.stroke();
  }
}

function drawAngler(ctx, x, footY) {
  ctx.fillStyle = '#3a4a6a';
  ctx.fillRect(x - 5, footY - 22, 10, 14);
  ctx.fillStyle = '#e0b48a';
  ctx.fillRect(x - 4, footY - 31, 8, 8);
  ctx.fillStyle = '#c0392b';
  ctx.fillRect(x - 5, footY - 33, 10, 3);
  // rod
  ctx.strokeStyle = '#caa14a';
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x + 4, footY - 20);
  ctx.lineTo(x + 42, footY - 32);
  ctx.stroke();
}

function drawBobber(ctx, x, y, dip) {
  ctx.fillStyle = '#e8554f';
  ctx.fillRect(x - 4, y - 4 + dip, 8, 5);
  ctx.fillStyle = '#f5f1e4';
  ctx.fillRect(x - 4, y + 1 + dip, 8, 4);
}

// A pixel fish; facing -1 (left) or 1 (right). r is body half-length.
function drawFish(ctx, x, y, r, color, facing) {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(facing, 1);
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.ellipse(0, 0, r, r * 0.62, 0, 0, Math.PI * 2);
  ctx.fill();
  // tail
  ctx.beginPath();
  ctx.moveTo(-r * 0.85, 0);
  ctx.lineTo(-r * 1.5, -r * 0.6);
  ctx.lineTo(-r * 1.5, r * 0.6);
  ctx.closePath();
  ctx.fill();
  // belly shine
  ctx.fillStyle = 'rgba(255,255,255,0.32)';
  ctx.fillRect(-r * 0.3, -r * 0.42, r * 0.9, r * 0.22);
  // eye
  ctx.fillStyle = '#10141c';
  ctx.fillRect(r * 0.45, -r * 0.22, Math.max(2, r * 0.2), Math.max(2, r * 0.2));
  ctx.restore();
}
