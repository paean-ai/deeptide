// Pixel Connect Four - backdrop, board and disc rendering.

const DISC_COLORS = { 1: '#e8554f', 2: '#f2cf3f' };
const DISC_SHADE = { 1: '#9c2f2c', 2: '#a98a1e' };

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1c2848');
  g.addColorStop(1, '#070a12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawDisc(ctx, cx, cy, r, player, highlight) {
  ctx.fillStyle = DISC_SHADE[player];
  ctx.beginPath();
  ctx.arc(cx, cy + 2, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = DISC_COLORS[player];
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = 'rgba(255,255,255,0.32)';
  ctx.beginPath();
  ctx.arc(cx - r * 0.32, cy - r * 0.32, r * 0.34, 0, Math.PI * 2);
  ctx.fill();
  if (highlight) {
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(cx, cy, r + 2, 0, Math.PI * 2);
    ctx.stroke();
  }
}

// The blue board: a panel with a circular hole per cell. Holes show the disc
// underneath, or an empty dark socket.
function drawBoard(ctx, gx, gy, cell, board, fallingCol) {
  if (fallingCol >= 0) {
    ctx.fillStyle = 'rgba(255,255,255,0.07)';
    ctx.fillRect(gx + fallingCol * cell, gy - 6, cell, ROWS * cell + 6);
  }
  ctx.fillStyle = '#2f5bd0';
  ctx.fillRect(gx, gy, cell * COLS, cell * ROWS);
  ctx.fillStyle = '#234aae';
  ctx.fillRect(gx, gy, cell * COLS, 4);
  const r = cell * 0.4;
  for (let row = 0; row < ROWS; row++) {
    for (let c = 0; c < COLS; c++) {
      const cx = gx + c * cell + cell / 2, cy = gy + row * cell + cell / 2;
      const v = board[row][c];
      if (v === 0) {
        ctx.fillStyle = '#0e1834';
        ctx.beginPath();
        ctx.arc(cx, cy, r, 0, Math.PI * 2);
        ctx.fill();
      } else {
        drawDisc(ctx, cx, cy, r, v, false);
      }
    }
  }
}
