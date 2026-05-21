// Pixel-art rendering for Pixel Bucket Brigade. 360x480 world units.

const PALETTE = {
  skyTop:   '#2a3358',
  skyLow:   '#5a6488',
  ground:   '#3a3048',
  groundHi: '#4c4060',
  bomber:   '#2a2436',
  bomberHi: '#4a4258',
  bomberEye:'#ff5a4a',
  bombBody: '#23222c',
  bombHi:   '#4a4a58',
  fuse:     '#8a6a3a',
  spark:    '#ffd24a',
  fast:     '#e8554f',
  cluster:  '#a06fd0',
  gold:     '#f4c44a',
  bucket:   '#c8722e',
  bucketHi: '#e8a060',
  bucketDk: '#7a3e16',
  band:     '#d8d2c0',
  rim:      '#ffe2a0',
  magnet:   'rgba(95,134,224,0.4)',
  pwBucket: '#5fc06e',
  pwSlow:   '#46b8e8',
  pwMagnet: '#5f86e0',
  hud:      '#10131f',
  hudText:  '#f3f1e6',
  heart:    '#ff5a6a',
  accent:   '#f4c44a',
};

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, PALETTE.skyTop);
  g.addColorStop(1, PALETTE.skyLow);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // Stars.
  ctx.fillStyle = 'rgba(255,255,255,0.5)';
  for (let i = 0; i < 36; i++) ctx.fillRect((i * 71 + 19) % VW, (i * 43 + 11) % 360, 2, 2);
  // Ground.
  ctx.fillStyle = PALETTE.ground;
  ctx.fillRect(0, 414, VW, VH - 414);
  ctx.fillStyle = PALETTE.groundHi;
  ctx.fillRect(0, 414, VW, 3);
}

function drawBomber(ctx, s) {
  const x = s.bomber.x, y = BOMBER_Y;
  // Body.
  ctx.fillStyle = PALETTE.bomber;
  ctx.fillRect((x - 16) | 0, (y - 14) | 0, 32, 26);
  ctx.fillStyle = PALETTE.bomberHi;
  ctx.fillRect((x - 16) | 0, (y - 14) | 0, 32, 3);
  // Eyes (look toward travel direction).
  ctx.fillStyle = PALETTE.bomberEye;
  const ex = s.bomber.dir > 0 ? 4 : -8;
  ctx.fillRect((x + ex) | 0, (y - 6) | 0, 4, 4);
  ctx.fillRect((x + ex + 7) | 0, (y - 6) | 0, 4, 4);
  // Arms.
  ctx.fillStyle = PALETTE.bomber;
  ctx.fillRect((x - 22) | 0, (y - 2) | 0, 7, 5);
  ctx.fillRect((x + 15) | 0, (y - 2) | 0, 7, 5);
}

function bombColor(kind) {
  if (kind === B_FAST)    return PALETTE.fast;
  if (kind === B_CLUSTER) return PALETTE.cluster;
  if (kind === B_GOLD)    return PALETTE.gold;
  return PALETTE.bombBody;
}

function drawBombs(ctx, s) {
  for (const b of s.bombs) {
    const col = bombColor(b.kind);
    // Fuse + spark.
    ctx.fillStyle = PALETTE.fuse;
    ctx.fillRect((b.x - 1) | 0, (b.y - 13) | 0, 2, 6);
    ctx.fillStyle = PALETTE.spark;
    ctx.fillRect((b.x - 2) | 0, (b.y - 16) | 0, 4, 4);
    // Body.
    ctx.fillStyle = col;
    ctx.beginPath(); ctx.arc(b.x, b.y, 8, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = PALETTE.bombHi;
    ctx.fillRect((b.x - 4) | 0, (b.y - 5) | 0, 3, 3);
  }
}

function drawPowerups(ctx, s) {
  for (const p of s.powerups) {
    const col = p.kind === P_BUCKET ? PALETTE.pwBucket
              : p.kind === P_SLOW ? PALETTE.pwSlow : PALETTE.pwMagnet;
    ctx.fillStyle = '#10131f';
    ctx.fillRect((p.x - 10) | 0, (p.y - 10) | 0, 20, 20);
    ctx.fillStyle = col;
    ctx.fillRect((p.x - 8) | 0, (p.y - 8) | 0, 16, 16);
    ctx.fillStyle = '#10131f';
    const sym = p.kind === P_BUCKET ? '+' : p.kind === P_SLOW ? 'S' : 'M';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(sym, p.x, p.y + 1);
    ctx.textBaseline = 'alphabetic';
  }
}

function drawStack(ctx, s) {
  const x = s.stackX;
  const halfTop = (s.magnet > 0 ? MAGNET_HALF : CATCH_HALF);
  // Magnet aura on the catching rim.
  if (s.magnet > 0) {
    ctx.fillStyle = PALETTE.magnet;
    ctx.fillRect((x - MAGNET_HALF - 4) | 0, RIM_Y - 6, (MAGNET_HALF + 4) * 2, 12);
  }
  // Buckets, top one is the catch bucket.
  for (let i = s.buckets - 1; i >= 0; i--) {
    const top = RIM_Y + i * 5;
    const hw = i === 0 ? halfTop : CATCH_HALF - 3;
    ctx.fillStyle = PALETTE.bucketDk;
    ctx.fillRect((x - hw) | 0, top, hw * 2, 22);
    ctx.fillStyle = PALETTE.bucket;
    ctx.fillRect((x - hw + 2) | 0, top + 2, hw * 2 - 4, 18);
    ctx.fillStyle = PALETTE.band;
    ctx.fillRect((x - hw + 2) | 0, top + 7, hw * 2 - 4, 3);
    ctx.fillStyle = PALETTE.bucketHi;
    ctx.fillRect((x - hw + 2) | 0, top + 2, 3, 18);
  }
  // Bright rim on the catch bucket.
  ctx.fillStyle = PALETTE.rim;
  ctx.fillRect((x - halfTop) | 0, RIM_Y - 2, halfTop * 2, 3);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 28);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 11px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'wave') + ' ' + s.wave, 8, 14);
  for (let i = 0; i < s.buckets; i++) drawBucketPip(ctx, 78 + i * 12, 14);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 8, 14);
  // Active power-up timers.
  ctx.textAlign = 'center';
  ctx.font = 'bold 10px monospace';
  let tag = '';
  if (s.slow > 0)   tag += t(lang, 'slow') + ' ';
  if (s.magnet > 0) tag += t(lang, 'magnet');
  if (tag) { ctx.fillStyle = PALETTE.accent; ctx.fillText(tag, VW / 2, 14); }
}

function drawBucketPip(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.bucket;
  ctx.fillRect(cx - 4, cy - 4, 8, 8);
  ctx.fillStyle = PALETTE.band;
  ctx.fillRect(cx - 4, cy - 1, 8, 2);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  ctx.fillStyle = 'rgba(255,90,74,' + Math.min(0.5, s.flash) + ')';
  ctx.fillRect(0, 28, VW, VH - 28);
}

function drawWaveBanner(ctx, lang, s) {
  if (s.waveBanner <= 0) return;
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 20px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'wave') + ' ' + s.wave, VW / 2, 220);
}

function drawTitleArt(ctx, cx, cy) {
  // A few bombs raining toward a bucket stack.
  for (const [dx, dy, k] of [[-40, -34, B_NORMAL], [4, 10, B_FAST], [44, -52, B_GOLD]]) {
    ctx.fillStyle = PALETTE.spark;
    ctx.fillRect(cx + dx - 2, cy + dy - 16, 4, 4);
    ctx.fillStyle = bombColor(k);
    ctx.beginPath(); ctx.arc(cx + dx, cy + dy, 8, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = PALETTE.bombHi;
    ctx.fillRect(cx + dx - 4, cy + dy - 5, 3, 3);
  }
  drawStack(ctx, { stackX: cx, buckets: 3, magnet: 0 });
}
