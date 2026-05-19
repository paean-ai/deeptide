// Pixel Codebreaker - backdrop and peg rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#2a2046');
  g.addColorStop(1, '#09070f');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A code peg. colorIdx -1 = empty socket.
function drawPeg(ctx, cx, cy, r, colorIdx) {
  if (colorIdx < 0) {
    ctx.fillStyle = '#171127';
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#39305a';
    ctx.lineWidth = 1.5;
    ctx.stroke();
    return;
  }
  ctx.fillStyle = 'rgba(0,0,0,0.4)';
  ctx.beginPath();
  ctx.arc(cx, cy + 2, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = PEG_COLORS[colorIdx];
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = 'rgba(255,255,255,0.34)';
  ctx.beginPath();
  ctx.arc(cx - r * 0.32, cy - r * 0.32, r * 0.36, 0, Math.PI * 2);
  ctx.fill();
}

// Feedback cluster: `black` solid white pegs, `white` hollow pegs, rest blank.
function drawFeedback(ctx, x, y, total, black, white) {
  const per = Math.ceil(total / 2);
  const dot = 5;
  for (let i = 0; i < total; i++) {
    const col = i % per, row = (i / per) | 0;
    const dx = x + col * (dot * 2 + 2), dy = y + row * (dot * 2 + 2);
    if (i < black) {
      ctx.fillStyle = '#f5f1e4';
      ctx.beginPath();
      ctx.arc(dx, dy, dot, 0, Math.PI * 2);
      ctx.fill();
    } else if (i < black + white) {
      ctx.strokeStyle = '#f5f1e4';
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.arc(dx, dy, dot - 1, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      ctx.fillStyle = '#241b3c';
      ctx.beginPath();
      ctx.arc(dx, dy, dot - 1, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}
