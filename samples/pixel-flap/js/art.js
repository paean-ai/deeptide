// Pixel Flap - rendering.

const COL = {
  sky1: '#5fc0e8', sky2: '#a8e2f0', cloud: 'rgba(255,255,255,0.5)',
  ground: '#c9a35a', groundEdge: '#7a5a30', grass: '#3fa040',
  pipe: '#3fa040', pipeDark: '#2a6a26', pipeLip: '#1a5a18',
  bird: '#ffd23e', birdDark: '#a06010', birdWing: '#ffb45a',
  birdEye: '#1a1a1a', birdBeak: '#ff7a3a',
  hint: '#ffe07a',
};

const CLOUDS = (() => {
  const out = []; let s = 8888;
  for (let i = 0; i < 6; i++) {
    s = (s * 16807) % 2147483647;
    out.push({ x: s % VW, y: 50 + (s * 7) % 200, w: 30 + (s % 30) });
  }
  return out;
})();

function drawBackground(ctx, scroll) {
  const g = ctx.createLinearGradient(0, 0, 0, GROUND);
  g.addColorStop(0, COL.sky1);
  g.addColorStop(1, COL.sky2);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, GROUND);
  ctx.fillStyle = COL.cloud;
  for (const c of CLOUDS) {
    const x = ((c.x - scroll * 0.2) % (VW + c.w * 2) + VW + c.w * 2) % (VW + c.w * 2) - c.w;
    ctx.fillRect(x, c.y, c.w, 8);
    ctx.fillRect(x + 6, c.y - 6, c.w - 12, 6);
  }
  // ground
  ctx.fillStyle = COL.ground;
  ctx.fillRect(0, GROUND, VW, VH - GROUND);
  ctx.fillStyle = COL.grass;
  ctx.fillRect(0, GROUND, VW, 5);
  ctx.fillStyle = COL.groundEdge;
  ctx.fillRect(0, GROUND + 5, VW, 2);
}

function drawWorld(ctx, s, clock) {
  // pipes
  for (const p of s.pipes) {
    if (p.x + PIPE_W < 0 || p.x > VW) continue;
    // top pipe
    drawPipe(ctx, p.x, CEIL, PIPE_W, p.gapY - CEIL, false);
    // bottom pipe
    drawPipe(ctx, p.x, p.gapY + p.gapH, PIPE_W, GROUND - (p.gapY + p.gapH), true);
  }
  // bird
  if (s.bird.alive) drawBird(ctx, BIRD_X, s.bird.y, s.bird.vy, clock);
}

function drawPipe(ctx, x, y, w, h, isBottom) {
  ctx.fillStyle = COL.pipe;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = COL.pipeDark;
  ctx.fillRect(x, y, 4, h);
  ctx.fillRect(x + w - 4, y, 4, h);
  // lip
  const lipY = isBottom ? y : y + h - 10;
  ctx.fillStyle = COL.pipe;
  ctx.fillRect(x - 3, lipY, w + 6, 10);
  ctx.fillStyle = COL.pipeLip;
  ctx.fillRect(x - 3, lipY, w + 6, 2);
  ctx.fillStyle = COL.pipeDark;
  ctx.fillRect(x - 3, lipY, 3, 10);
  ctx.fillRect(x + w, lipY, 3, 10);
}

function drawBird(ctx, x, y, vy, clock) {
  const angle = Math.max(-0.5, Math.min(0.9, vy / 600));
  const wingFlap = Math.sin(clock * 18) * 0.4;
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(angle);
  // body
  ctx.fillStyle = COL.birdDark;
  ctx.fillRect(-BIRD_R, -BIRD_R + 3, BIRD_R * 2, BIRD_R * 2 - 2);
  ctx.fillStyle = COL.bird;
  ctx.fillRect(-BIRD_R, -BIRD_R, BIRD_R * 2, BIRD_R * 2 - 4);
  // wing
  ctx.fillStyle = COL.birdWing;
  ctx.fillRect(-BIRD_R + 1, 1 + wingFlap * 2, BIRD_R, BIRD_R - 2);
  // eye
  ctx.fillStyle = COL.birdEye;
  ctx.fillRect(BIRD_R / 2 + 1, -BIRD_R / 2, 3, 3);
  // beak
  ctx.fillStyle = COL.birdBeak;
  ctx.fillRect(BIRD_R - 1, -1, 6, 4);
  ctx.restore();
}

function drawTapHint(ctx, clock) {
  const pulse = 0.6 + 0.4 * Math.sin(clock * 4);
  ctx.fillStyle = `rgba(255,224,122,${pulse})`;
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(t('tapStart'), VW / 2, GROUND / 2 + 70);
}
