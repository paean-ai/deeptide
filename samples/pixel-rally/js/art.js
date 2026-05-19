// Pixel Rally - rendering.

const COL = {
  court: '#0a1428', courtEdge: '#2a4068', mid: '#5fc6e8',
  player: '#7fffd4', cpu: '#ff6e7a', ball: '#ffe07a',
  serve: '#ffe07a',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#0c1a30');
  g.addColorStop(1, '#04070f');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawCourt(ctx, s) {
  ctx.fillStyle = COL.court;
  ctx.fillRect(0, COURT_TOP, VW, COURT_BOTTOM - COURT_TOP);
  ctx.strokeStyle = COL.courtEdge;
  ctx.lineWidth = 2;
  ctx.strokeRect(1, COURT_TOP + 1, VW - 2, COURT_BOTTOM - COURT_TOP - 2);
  // midline (dashed)
  ctx.fillStyle = COL.mid;
  const my = (COURT_TOP + COURT_BOTTOM) / 2;
  for (let x = 8; x < VW - 8; x += 14) ctx.fillRect(x, my - 1, 9, 2);
  // big centred score
  ctx.fillStyle = 'rgba(255,255,255,0.06)';
  ctx.font = 'bold 80px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(s.cpuScore + ' : ' + s.playerScore, VW / 2, my);
}

function drawPaddles(ctx, s) {
  ctx.fillStyle = COL.cpu;
  ctx.fillRect(s.cpuX - PADDLE_W / 2, CPU_Y - PADDLE_H / 2, PADDLE_W, PADDLE_H);
  ctx.fillStyle = COL.player;
  ctx.fillRect(s.playerX - PADDLE_W / 2, PLAYER_Y - PADDLE_H / 2, PADDLE_W, PADDLE_H);
}

function drawBall(ctx, s) {
  if (s.serveT > 0) return;
  ctx.fillStyle = COL.ball;
  const r = BALL_R;
  ctx.fillRect(Math.round(s.ball.x - r), Math.round(s.ball.y - r), r * 2, r * 2);
}

function drawServeHint(ctx, s) {
  if (s.serveT <= 0 || s.over) return;
  ctx.fillStyle = COL.serve;
  ctx.font = 'bold 11px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(t('serve') + ' ' + Math.max(1, Math.ceil(s.serveT)),
    VW / 2, (COURT_TOP + COURT_BOTTOM) / 2 + 50);
}

function drawAll(ctx, s) {
  drawBackground(ctx);
  drawCourt(ctx, s);
  drawPaddles(ctx, s);
  drawBall(ctx, s);
  drawServeHint(ctx, s);
}
