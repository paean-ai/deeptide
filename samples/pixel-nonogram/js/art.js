// Pixel Nonogram - palette + grid / clue rendering helpers.

// 3x4 pixel font for clue digits 0-9.
const CLUE_DIGITS = {
  0: ['111', '101', '101', '111'],
  1: ['010', '110', '010', '111'],
  2: ['111', '011', '110', '111'],
  3: ['111', '011', '001', '111'],
  4: ['101', '101', '111', '001'],
  5: ['111', '110', '011', '111'],
  6: ['111', '100', '111', '111'],
  7: ['111', '001', '010', '010'],
  8: ['111', '111', '101', '111'],
  9: ['111', '111', '011', '111'],
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1b2030');
  g.addColorStop(1, '#0a0c14');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// Draw an integer right-anchored at (rx, ty) — rx is the right edge.
function drawClueNumber(ctx, n, rx, ty, px, color) {
  const digits = String(n).split('');
  let x = rx - digits.length * 4 * px + px;
  ctx.fillStyle = color;
  for (const d of digits) {
    const rows = CLUE_DIGITS[d];
    for (let r = 0; r < 4; r++) {
      for (let c = 0; c < 3; c++) {
        if (rows[r][c] === '1') ctx.fillRect(x + c * px, ty + r * px, px, px);
      }
    }
    x += 4 * px;
  }
}

// One nonogram cell at (x, y, s) for state 0 (blank) / 1 (filled) / 2 (marked).
function drawCell(ctx, x, y, s, st, color, popScale) {
  ctx.fillStyle = '#1d2336';
  ctx.fillRect(x, y, s, s);
  if (st === 1) {
    const inset = (1 - popScale) * s * 0.5;
    const cs = s - inset * 2;
    ctx.fillStyle = color;
    ctx.fillRect(x + inset, y + inset, cs, cs);
    ctx.fillStyle = 'rgba(255, 255, 255, 0.22)';
    ctx.fillRect(x + inset, y + inset, cs, Math.max(1, cs * 0.18));
  } else if (st === 2) {
    // a small pixel cross marking a "definitely empty" cell
    const m = Math.round(s * 0.3);
    const w = Math.max(2, Math.round(s * 0.1));
    ctx.strokeStyle = '#5d6982';
    ctx.lineWidth = w;
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(x + m, y + m);
    ctx.lineTo(x + s - m, y + s - m);
    ctx.moveTo(x + s - m, y + m);
    ctx.lineTo(x + m, y + s - m);
    ctx.stroke();
  }
}
