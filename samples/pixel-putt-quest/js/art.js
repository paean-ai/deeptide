// Pixel Putt Quest - course, terrain, ball, cup and aim rendering.

function drawCourse(ctx, hole, t) {
  // grass
  ctx.fillStyle = '#2f7d3a';
  ctx.fillRect(0, 0, VW, VH);
  for (let y = 0; y < VH; y += 16) {
    for (let x = 0; x < VW; x += 16) {
      if ((x / 16 + y / 16) % 2 === 0) { ctx.fillStyle = '#338544'; ctx.fillRect(x, y, 16, 16); }
    }
  }
  // water
  for (const w of hole.water) {
    ctx.fillStyle = '#2f6fd0';
    ctx.fillRect(w[0], w[1], w[2], w[3]);
    ctx.fillStyle = '#4f8fe8';
    for (let i = 0; i < w[2]; i += 14) {
      ctx.fillRect(w[0] + ((i + t * 16) % w[2]), w[1] + 6 + (i % 3) * 9, 8, 2);
    }
    ctx.fillStyle = '#1f4f9a';
    ctx.fillRect(w[0], w[1], w[2], 3);
  }
  // sand
  for (const s of hole.sand) {
    ctx.fillStyle = '#d8c068';
    ctx.fillRect(s[0], s[1], s[2], s[3]);
    ctx.fillStyle = '#c4a850';
    for (let i = 4; i < s[2]; i += 13) ctx.fillRect(s[0] + i, s[1] + (i % 17) + 4, 3, 3);
  }
}

function drawWalls(ctx, walls) {
  for (const w of walls) {
    ctx.fillStyle = '#5a3f28';
    ctx.fillRect(w[0], w[1], w[2], w[3]);
    ctx.fillStyle = '#7a5736';
    ctx.fillRect(w[0], w[1], w[2], Math.min(4, w[3]));
    ctx.fillStyle = '#3c2a1a';
    ctx.fillRect(w[0], w[1] + w[3] - Math.min(4, w[3]), w[2], Math.min(4, w[3]));
  }
}

function drawCup(ctx, cup, t) {
  ctx.fillStyle = '#10160d';
  ctx.beginPath();
  ctx.arc(cup.x, cup.y, CUP_R, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#1c2614';
  ctx.beginPath();
  ctx.arc(cup.x, cup.y, CUP_R - 3, 0, Math.PI * 2);
  ctx.fill();
  // flag
  ctx.fillStyle = '#e8e3d6';
  ctx.fillRect(cup.x - 1, cup.y - 46, 3, 46);
  ctx.fillStyle = '#e0463f';
  const wave = Math.sin(t * 4) * 2;
  ctx.fillRect(cup.x + 2, cup.y - 46, 20, 12 + wave);
  ctx.fillStyle = '#b8342f';
  ctx.fillRect(cup.x + 2, cup.y - 46, 20, 3);
}

function drawBall(ctx, x, y) {
  ctx.fillStyle = 'rgba(0,0,0,0.3)';
  ctx.beginPath(); ctx.ellipse(x + 2, y + 3, BALL_R, BALL_R * 0.7, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#f4f7ff';
  ctx.beginPath(); ctx.arc(x, y, BALL_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#c9d2e0';
  ctx.beginPath(); ctx.arc(x + 1.5, y + 1.5, BALL_R - 2, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(x - 3, y - 4, 3, 3);
}

// Aim guide: a dotted line from the ball opposite the drag, plus a power bar.
function drawAim(ctx, ball, dragX, dragY) {
  const dx = ball.x - dragX, dy = ball.y - dragY;
  const len = Math.hypot(dx, dy);
  if (len < 4) return;
  const power = Math.min(1, len / DRAG_MAX);
  const ux = dx / len, uy = dy / len;
  for (let i = 1; i <= 12; i++) {
    const d = i * 13;
    if (d > 60 + power * 120) break;
    ctx.fillStyle = i % 2 ? '#ffffff' : '#ffe14d';
    ctx.fillRect(ball.x + ux * d - 2, ball.y + uy * d - 2, 4, 4);
  }
  // power ring on the ball
  ctx.strokeStyle = power > 0.85 ? '#ff5d5d' : '#ffe14d';
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.arc(ball.x, ball.y, BALL_R + 5, -Math.PI / 2, -Math.PI / 2 + power * Math.PI * 2);
  ctx.stroke();
}
