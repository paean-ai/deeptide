// Pixel Unblock - backdrop and board / block rendering.

const BLOCK_COLORS = ['#5aa9e8', '#5fc06e', '#f2cf3f', '#9a6cd8',
                      '#ef9b3e', '#4fd6d6', '#ff7db0', '#a8d84a', '#c8804a', '#7f93b8'];

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#322624');
  g.addColorStop(1, '#0e0a0a');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// The board frame, with a gap in the right wall on the exit row.
function drawBoard(ctx, gx, gy, cell, exitRow) {
  const size = cell * GRID_N;
  ctx.fillStyle = '#1b1412';
  ctx.fillRect(gx, gy, size, size);
  // cell grid
  ctx.strokeStyle = '#2e2422';
  ctx.lineWidth = 1;
  for (let i = 1; i < GRID_N; i++) {
    ctx.beginPath();
    ctx.moveTo(gx + i * cell, gy); ctx.lineTo(gx + i * cell, gy + size);
    ctx.moveTo(gx, gy + i * cell); ctx.lineTo(gx + size, gy + i * cell);
    ctx.stroke();
  }
  // walls (skip the exit gap on the right)
  ctx.fillStyle = '#4a3832';
  const w = 5;
  ctx.fillRect(gx - w, gy - w, size + 2 * w, w);
  ctx.fillRect(gx - w, gy + size, size + 2 * w, w);
  ctx.fillRect(gx - w, gy - w, w, size + 2 * w);
  ctx.fillRect(gx + size, gy - w, w, exitRow * cell + w);
  ctx.fillRect(gx + size, gy + (exitRow + 1) * cell, w, size - (exitRow + 1) * cell + w);
  // exit arrow
  ctx.fillStyle = '#e8554f';
  const ey = gy + (exitRow + 0.5) * cell;
  ctx.beginPath();
  ctx.moveTo(gx + size + 12, ey);
  ctx.lineTo(gx + size + 4, ey - 6);
  ctx.lineTo(gx + size + 4, ey + 6);
  ctx.closePath();
  ctx.fill();
}

function drawBlock(ctx, x, y, w, h, color, dragging) {
  ctx.fillStyle = color;
  ctx.fillRect(x + 2, y + 2, w - 4, h - 4);
  ctx.fillStyle = 'rgba(255,255,255,0.26)';
  ctx.fillRect(x + 2, y + 2, w - 4, 4);
  ctx.fillStyle = 'rgba(0,0,0,0.3)';
  ctx.fillRect(x + 2, y + h - 6, w - 4, 4);
  if (dragging) {
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 3, y + 3, w - 6, h - 6);
  }
}

// The target block gets headlights so its exit direction reads.
function drawTarget(ctx, x, y, w, h, dragging) {
  drawBlock(ctx, x, y, w, h, '#e8554f', dragging);
  ctx.fillStyle = '#ffe2a0';
  ctx.fillRect(x + w - 9, y + 7, 4, 4);
  ctx.fillRect(x + w - 9, y + h - 11, 4, 4);
}
