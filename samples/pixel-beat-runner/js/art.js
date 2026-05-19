// Pixel Beat Runner - lane highway, notes, hit effects.

function brShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

function drawHighway(ctx, t, lanePulse) {
  ctx.fillStyle = '#0c0a16';
  ctx.fillRect(0, 0, VW, VH);
  for (let i = 0; i < LANES; i++) {
    const x = i * LANE_W;
    ctx.fillStyle = i % 2 ? '#13111f' : '#171426';
    ctx.fillRect(x, 0, LANE_W, VH);
    // lane lit flash when struck
    if (lanePulse[i] > 0) {
      ctx.globalAlpha = lanePulse[i] * 0.32;
      ctx.fillStyle = LANE_COLOR[i];
      ctx.fillRect(x, 0, LANE_W, VH);
      ctx.globalAlpha = 1;
    }
    ctx.fillStyle = '#2a2640';
    ctx.fillRect(x, 0, 2, VH);
  }
  // judgement line
  ctx.fillStyle = '#3a3658';
  ctx.fillRect(0, HIT_Y - 3, VW, 6);
  for (let i = 0; i < LANES; i++) {
    const cx = i * LANE_W + LANE_W / 2;
    ctx.strokeStyle = LANE_COLOR[i];
    ctx.lineWidth = 3;
    ctx.strokeRect(cx - 26, HIT_Y - 26, 52, 52);
  }
}

function drawNote(ctx, lane, y) {
  const cx = lane * LANE_W + LANE_W / 2;
  const col = LANE_COLOR[lane];
  const w = 54, h = 30;
  ctx.fillStyle = brShade(col, -70);
  ctx.fillRect(cx - w / 2, y - h / 2, w, h);
  ctx.fillStyle = col;
  ctx.fillRect(cx - w / 2 + 3, y - h / 2 + 3, w - 6, h - 6);
  ctx.fillStyle = brShade(col, 60);
  ctx.fillRect(cx - w / 2 + 3, y - h / 2 + 3, w - 6, 5);
  ctx.fillStyle = '#fff';
  ctx.globalAlpha = 0.5;
  ctx.fillRect(cx - 5, y - 5, 10, 10);
  ctx.globalAlpha = 1;
}

function drawHitFx(ctx, fx) {
  const cx = fx.lane * LANE_W + LANE_W / 2;
  const r = (1 - fx.life) * 34 + 8;
  ctx.globalAlpha = Math.max(0, fx.life);
  ctx.strokeStyle = fx.color;
  ctx.lineWidth = 3;
  ctx.strokeRect(cx - r, HIT_Y - r, r * 2, r * 2);
  ctx.globalAlpha = 1;
}
