// Pixel-art rendering for Pixel Copter. 360x480 world units.

const PALETTE = {
  skyTop: '#1a2548',
  skyMid: '#2c5085',
  skyBot: '#558ac7',
  star:   '#dde6ff',
  caveFill:    '#3a2a18',
  caveEdge:    '#7d4f29',
  caveShade:   '#5a3819',
  pillar:      '#a36835',
  pillarShade: '#7d4f29',
  copter:      '#e8554f',
  copterDark:  '#a8373a',
  copterTail:  '#f8f5e8',
  rotor:       '#9aa6cc',
  glass:       '#cce6ff',
  hud:         '#0d1228',
  hudText:     '#f8f5e8',
  hudDim:      '#9aa6cc',
  ok:          '#54c47c',
};

function drawScene(ctx, s, viewW, viewH, lang, best) {
  drawSky(ctx, s, viewW, viewH);
  drawCave(ctx, s);
  drawPillars(ctx, s);
  drawCopter(ctx, s);
  drawHud(ctx, s, lang, best);
}

function drawSky(ctx, s, w, h) {
  const grad = ctx.createLinearGradient(0, 32, 0, h);
  grad.addColorStop(0, PALETTE.skyTop);
  grad.addColorStop(0.5, PALETTE.skyMid);
  grad.addColorStop(1, PALETTE.skyBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 32, w, h - 32);
  // Tiny scrolling stars for parallax.
  ctx.fillStyle = PALETTE.star;
  const off = ((s.distance * 0.5) | 0) % w;
  for (let i = 0; i < 30; i++) {
    const sx = ((i * 41 + 9) - off + w) % w;
    const sy = 40 + (i * 31 % (h - 80));
    ctx.fillRect(sx, sy, 1, 1);
  }
}

function drawCave(ctx, s) {
  // Top boundary polygon.
  ctx.fillStyle = PALETTE.caveFill;
  ctx.beginPath();
  ctx.moveTo(-10, 32);
  for (const samp of s.samples) {
    const top = samp.midY - samp.gap / 2;
    ctx.lineTo(samp.x, top);
  }
  ctx.lineTo(370, 32);
  ctx.closePath();
  ctx.fill();
  // Top edge line.
  ctx.strokeStyle = PALETTE.caveEdge;
  ctx.lineWidth = 2;
  ctx.beginPath();
  for (let i = 0; i < s.samples.length; i++) {
    const samp = s.samples[i];
    const top = samp.midY - samp.gap / 2;
    if (i === 0) ctx.moveTo(samp.x, top); else ctx.lineTo(samp.x, top);
  }
  ctx.stroke();
  // Bottom boundary polygon.
  ctx.fillStyle = PALETTE.caveFill;
  ctx.beginPath();
  ctx.moveTo(-10, 480);
  for (const samp of s.samples) {
    const bot = samp.midY + samp.gap / 2;
    ctx.lineTo(samp.x, bot);
  }
  ctx.lineTo(370, 480);
  ctx.closePath();
  ctx.fill();
  // Bottom edge line.
  ctx.strokeStyle = PALETTE.caveEdge;
  ctx.beginPath();
  for (let i = 0; i < s.samples.length; i++) {
    const samp = s.samples[i];
    const bot = samp.midY + samp.gap / 2;
    if (i === 0) ctx.moveTo(samp.x, bot); else ctx.lineTo(samp.x, bot);
  }
  ctx.stroke();
}

function drawPillars(ctx, s) {
  for (const p of s.pillars) {
    const gapTop = p.gapY - p.gapH / 2;
    const gapBot = p.gapY + p.gapH / 2;
    ctx.fillStyle = PALETTE.pillarShade;
    ctx.fillRect(p.x, 32, 28, gapTop - 32);
    ctx.fillRect(p.x, gapBot, 28, 480 - gapBot);
    ctx.fillStyle = PALETTE.pillar;
    ctx.fillRect(p.x + 2, 32, 24, gapTop - 32);
    ctx.fillRect(p.x + 2, gapBot, 24, 480 - gapBot);
    // Caps.
    ctx.fillStyle = PALETTE.caveEdge;
    ctx.fillRect(p.x - 2, gapTop - 4, 32, 4);
    ctx.fillRect(p.x - 2, gapBot,     32, 4);
  }
}

function drawCopter(ctx, s) {
  const cx = 80, cy = s.copter.y;
  if (!s.copter.alive) {
    // Crashed: tilted red wreckage.
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(0.6);
    ctx.fillStyle = PALETTE.copterDark;
    ctx.fillRect(-10, -4, 20, 8);
    ctx.restore();
    return;
  }
  // Tilt based on vy.
  const tilt = Math.max(-0.3, Math.min(0.3, s.copter.vy / 800));
  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate(tilt);
  // Tail boom.
  ctx.fillStyle = PALETTE.copterTail;
  ctx.fillRect(-14, -1, 8, 2);
  ctx.fillRect(-15, -3, 2, 6);
  // Body.
  ctx.fillStyle = PALETTE.copterDark;
  ctx.fillRect(-8, -5, 16, 10);
  ctx.fillStyle = PALETTE.copter;
  ctx.fillRect(-7, -4, 14, 8);
  // Cockpit window.
  ctx.fillStyle = PALETTE.glass;
  ctx.fillRect(2, -3, 4, 4);
  // Skids.
  ctx.fillStyle = PALETTE.copterDark;
  ctx.fillRect(-7, 5, 14, 1);
  ctx.fillRect(-7, 5, 1, 2);
  ctx.fillRect(6,  5, 1, 2);
  // Rotor blur.
  ctx.fillStyle = PALETTE.rotor;
  ctx.fillRect(-9, -7, 18, 1);
  if ((s.copter.rotor | 0) % 2 === 0) ctx.fillRect(-2, -8, 4, 2);
  ctx.restore();
}

function drawHud(ctx, s, lang, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'distance') + ' ' + (s.distance | 0), 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'best') + ' ' + (best | 0), 352, 16);
}
