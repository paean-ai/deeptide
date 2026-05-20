// Pixel-art rendering for Pixel Pool. 360x480 world units.

const PALETTE = {
  bg:       '#0d1228',
  rail:     '#5a3819',
  railHi:   '#7d4f29',
  felt:     '#2a6f48',
  feltShade:'#234d36',
  pocket:   '#070b16',
  pocketHi: '#3a4274',
  cue:      '#f8f5e8',
  cueShade: '#b9c0d4',
  aimGuide: '#f7e69a',
  aimMax:   '#e8554f',
  hud:      '#0d1228',
  hudText:  '#f8f5e8',
  hudDim:   '#9aa6cc',
  ok:       '#54c47c',
};

function drawScene(ctx, s, viewW, viewH, lang) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, viewW, viewH);
  drawTable(ctx);
  drawPockets(ctx);
  drawAimGuide(ctx, s);
  drawBalls(ctx, s);
  drawHud(ctx, s, lang);
}

function drawTable(ctx) {
  // Outer rail.
  ctx.fillStyle = PALETTE.railHi;
  ctx.fillRect(16, 76, 328, 328);
  ctx.fillStyle = PALETTE.rail;
  ctx.fillRect(18, 78, 324, 324);
  // Felt.
  ctx.fillStyle = PALETTE.felt;
  ctx.fillRect(24, 84, 312, 312);
  // Felt shade lines for depth.
  ctx.fillStyle = PALETTE.feltShade;
  for (let x = 36; x < 336; x += 24) ctx.fillRect(x, 84, 1, 312);
}

function drawPockets(ctx) {
  for (const p of POCKETS) {
    ctx.fillStyle = PALETTE.pocketHi;
    ctx.beginPath(); ctx.arc(p.x, p.y, 13, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = PALETTE.pocket;
    ctx.beginPath(); ctx.arc(p.x, p.y, 11, 0, Math.PI * 2); ctx.fill();
  }
}

function drawBalls(ctx, s) {
  for (const b of s.balls) {
    if (!b.alive) continue;
    drawBall(ctx, b);
  }
}
function drawBall(ctx, b) {
  // Ball shadow.
  ctx.fillStyle = 'rgba(0,0,0,0.25)';
  ctx.beginPath(); ctx.arc(b.x + 1, b.y + 2, 7, 0, Math.PI * 2); ctx.fill();
  // Ball body.
  ctx.fillStyle = (b.kind === 'cue') ? PALETTE.cueShade : darken(b.color);
  ctx.beginPath(); ctx.arc(b.x, b.y, 8, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = (b.kind === 'cue') ? PALETTE.cue : b.color;
  ctx.beginPath(); ctx.arc(b.x, b.y, 7, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = '#fff';
  ctx.fillRect(((b.x - 2) | 0), ((b.y - 4) | 0), 2, 2);
}
function darken(hex) {
  // Crude darken by halving each channel.
  const r = parseInt(hex.slice(1, 3), 16) >> 1;
  const g = parseInt(hex.slice(3, 5), 16) >> 1;
  const bb = parseInt(hex.slice(5, 7), 16) >> 1;
  return '#' + ((1 << 24) | (r << 16) | (g << 8) | bb).toString(16).slice(1);
}

function drawAimGuide(ctx, s) {
  if (!s.aim || s.state !== 'aim') return;
  const cue = s.balls[0];
  if (!cue.alive) return;
  const dx = s.aim.x - cue.x;
  const dy = s.aim.y - cue.y;
  const len = Math.hypot(dx, dy);
  if (len < 8) return;
  // Slingshot: dotted guide AWAY from cue (drag direction), plus an arrow
  // line OPPOSITE the drag showing where the ball will roll.
  const ux = dx / len, uy = dy / len;
  // Drag-back trail (dim yellow dots).
  ctx.fillStyle = PALETTE.aimGuide;
  for (let d = 8; d < Math.min(120, len); d += 6) {
    ctx.fillRect(((cue.x + ux * d - 1) | 0), ((cue.y + uy * d - 1) | 0), 2, 2);
  }
  // Shot direction line (where the ball will GO).
  const shotLen = Math.min(120, len) * 1.4;
  ctx.fillStyle = (len > 110) ? PALETTE.aimMax : PALETTE.aimGuide;
  for (let d = 12; d < shotLen; d += 8) {
    ctx.fillRect(((cue.x - ux * d - 1) | 0), ((cue.y - uy * d - 1) | 0), 2, 2);
  }
  // Power dot at the drag end.
  const pdx = cue.x + ux * Math.min(120, len);
  const pdy = cue.y + uy * Math.min(120, len);
  ctx.fillStyle = (len > 110) ? PALETTE.aimMax : PALETTE.aimGuide;
  ctx.fillRect((pdx - 3) | 0, (pdy - 3) | 0, 6, 6);
}

function drawHud(ctx, s, lang) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'strokes') + ' ' + s.strokes + '/' + s.cfg.strokes, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = s.fouls ? '#e8554f' : PALETTE.hudText;
  ctx.fillText(t(lang, 'fouls') + ' ' + s.fouls, 352, 16);
}
