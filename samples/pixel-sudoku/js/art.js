// Pixel Sudoku - palette, pixel-font digits and backdrop.

// 3x5 pixel font for digits 1-9.
const DIGITS = {
  1: ['010', '110', '010', '010', '111'],
  2: ['111', '001', '111', '100', '111'],
  3: ['111', '001', '111', '001', '111'],
  4: ['101', '101', '111', '001', '001'],
  5: ['111', '100', '111', '001', '111'],
  6: ['111', '100', '111', '101', '111'],
  7: ['111', '001', '010', '010', '010'],
  8: ['111', '101', '111', '101', '111'],
  9: ['111', '101', '111', '001', '111'],
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1c2440');
  g.addColorStop(1, '#080a12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A digit centred at (cx, cy); px is the size of one font pixel.
function drawDigit(ctx, n, cx, cy, px, color) {
  const rows = DIGITS[n];
  if (!rows) return;
  const ox = Math.round(cx - 1.5 * px);
  const oy = Math.round(cy - 2.5 * px);
  ctx.fillStyle = color;
  for (let y = 0; y < 5; y++) {
    for (let x = 0; x < 3; x++) {
      if (rows[y][x] === '1') ctx.fillRect(ox + x * px, oy + y * px, px, px);
    }
  }
}
