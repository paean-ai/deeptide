// Pixel Mine Sweeper - palette + pixel sprite drawing.

// Classic minesweeper number colours, tuned bright for the dark board.
const NUM_COLORS = ['', '#5aa9ff', '#5fd17a', '#ff6b6b', '#b98bff',
                    '#ffae4a', '#4fd6d6', '#eef2f7', '#9aa6b8'];

// 3x5 pixel font for the count digits 1-8.
const DIGITS = {
  1: ['010', '110', '010', '010', '111'],
  2: ['111', '001', '111', '100', '111'],
  3: ['111', '001', '111', '001', '111'],
  4: ['101', '101', '111', '001', '001'],
  5: ['111', '100', '111', '001', '111'],
  6: ['111', '100', '111', '101', '111'],
  7: ['111', '001', '010', '010', '010'],
  8: ['111', '101', '111', '101', '111'],
};

function drawBackground(ctx, w, h) {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, '#141c2e');
  g.addColorStop(1, '#070a12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
}

// Draw a count digit centred at (cx, cy); px is the size of one font pixel.
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

// A bevelled covered tile filling (x, y, s, s).
function drawCovered(ctx, x, y, s) {
  const b = Math.max(2, Math.round(s * 0.12));
  ctx.fillStyle = '#33455f';
  ctx.fillRect(x, y, s, s);
  ctx.fillStyle = '#48618a';
  ctx.fillRect(x, y, s, b);
  ctx.fillRect(x, y, b, s);
  ctx.fillStyle = '#1d2839';
  ctx.fillRect(x, y + s - b, s, b);
  ctx.fillRect(x + s - b, y, b, s);
}

// A sunken revealed tile filling (x, y, s, s); hot tints the detonated tile.
function drawRevealed(ctx, x, y, s, hot) {
  ctx.fillStyle = hot ? '#3a1620' : '#161d2b';
  ctx.fillRect(x, y, s, s);
  const b = Math.max(1, Math.round(s * 0.08));
  ctx.fillStyle = hot ? '#22101a' : '#0f1420';
  ctx.fillRect(x, y, s, b);
  ctx.fillRect(x, y, b, s);
}

// A spiked pixel mine centred at (cx, cy) with body radius r.
function drawMine(ctx, cx, cy, r, hot) {
  const sp = Math.max(2, Math.round(r * 0.34));
  ctx.fillStyle = hot ? '#2a1414' : '#0d1018';
  for (let a = 0; a < 8; a++) {
    const ang = a * Math.PI / 4;
    ctx.fillRect(Math.round(cx + Math.cos(ang) * r - sp / 2),
                 Math.round(cy + Math.sin(ang) * r - sp / 2), sp, sp);
  }
  ctx.beginPath();
  ctx.arc(cx, cy, r * 0.78, 0, Math.PI * 2);
  ctx.fill();
  const hp = Math.max(2, Math.round(r * 0.28));
  ctx.fillStyle = hot ? '#ff8a6b' : '#4d5b78';
  ctx.fillRect(Math.round(cx - r * 0.36), Math.round(cy - r * 0.36), hp, hp);
}

// A flag planted in tile (x, y, s, s); scale lets it bounce when placed.
function drawFlag(ctx, x, y, s, scale) {
  const cx = x + s / 2, cy = y + s / 2;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scale, scale);
  ctx.translate(-cx, -cy);
  const px = Math.max(1, Math.round(s / 14));
  ctx.fillStyle = '#10202f';
  ctx.fillRect(Math.round(x + s * 0.26), Math.round(y + s * 0.7), Math.round(s * 0.48), px * 2);
  ctx.fillRect(Math.round(x + s * 0.5 - px), Math.round(y + s * 0.22), px * 2, Math.round(s * 0.5));
  ctx.fillStyle = '#ff5544';
  ctx.beginPath();
  ctx.moveTo(x + s * 0.5 - px, y + s * 0.22);
  ctx.lineTo(x + s * 0.5 - px, y + s * 0.47);
  ctx.lineTo(x + s * 0.2, y + s * 0.345);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

// A red cross marking a wrongly-flagged tile when the field is exposed.
function drawWrongFlag(ctx, x, y, s) {
  const px = Math.max(2, Math.round(s * 0.13));
  ctx.strokeStyle = '#ff6b6b';
  ctx.lineWidth = px;
  ctx.lineCap = 'square';
  const m = s * 0.26;
  ctx.beginPath();
  ctx.moveTo(x + m, y + m);
  ctx.lineTo(x + s - m, y + s - m);
  ctx.moveTo(x + s - m, y + m);
  ctx.lineTo(x + m, y + s - m);
  ctx.stroke();
}
