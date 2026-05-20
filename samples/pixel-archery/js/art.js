// Pixel-art rendering for Pixel Archery. 360x480 world units.

const PALETTE = {
  skyTop:  '#bfd9f0',
  skyBot:  '#e9d4a3',
  grass:   '#5fa83a',
  grassHi: '#7dc24e',
  grassDk: '#3d6f24',
  bow:     '#7d4f29',
  bowHi:   '#a36835',
  string:  '#f8f5e8',
  arrow:   '#9aa6cc',
  fletch:  '#e8554f',
  arrowHead:'#bfc7e6',
  target0: '#f8f5e8',          // outer
  target1: '#0c1230',          // mid
  target2: '#4a9be8',          // inner
  target3: '#e8554f',          // bull
  targetStand: '#7d4f29',
  hud:     '#0d1228',
  hudText: '#f8f5e8',
  hudDim:  '#9aa6cc',
  aimGuide:'#f7e69a',
  aimMax:  '#e8554f',
  ok:      '#54c47c',
};

function drawScene(ctx, s, viewW, viewH, lang) {
  drawSky(ctx, viewW, viewH);
  drawGround(ctx);
  drawTarget(ctx, s);
  drawHits(ctx, s);
  drawBow(ctx, s);
  drawArrow(ctx, s);
  drawAimGuide(ctx, s);
  drawWindIcon(ctx, s, lang);
  drawHud(ctx, s, lang);
}

function drawSky(ctx, w, h) {
  const grad = ctx.createLinearGradient(0, 32, 0, h - 80);
  grad.addColorStop(0, PALETTE.skyTop);
  grad.addColorStop(1, PALETTE.skyBot);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 32, w, h - 32);
}
function drawGround(ctx) {
  ctx.fillStyle = PALETTE.grass;
  ctx.fillRect(0, 400, 360, 80);
  ctx.fillStyle = PALETTE.grassHi;
  for (let x = 4; x < 360; x += 14) ctx.fillRect(x, 400, 3, 2);
  ctx.fillStyle = PALETTE.grassDk;
  ctx.fillRect(0, 416, 360, 1);
}

function drawTarget(ctx, s) {
  const { tx, ty, r } = s.cfg;
  // Wooden stand.
  ctx.fillStyle = PALETTE.targetStand;
  ctx.fillRect((tx - 2) | 0, (ty + r) | 0, 4, 400 - (ty + r));
  ctx.fillRect((tx - 8) | 0, 398, 16, 2);
  // Rings.
  for (let i = 0; i < RINGS.length; i++) {
    const ring = RINGS[i];
    const rad = r * ring.rad;
    ctx.fillStyle = ringColor(i);
    ctx.beginPath(); ctx.arc(tx, ty, rad, 0, Math.PI * 2); ctx.fill();
  }
  // Centre dot.
  ctx.fillStyle = '#fff';
  ctx.fillRect((tx | 0) - 1, (ty | 0) - 1, 2, 2);
}
function ringColor(i) {
  // RINGS index 0 is the bull (innermost) when drawn last; we draw from
  // largest to smallest, so reverse the colour selection.
  const palette = [PALETTE.target3, PALETTE.target2, PALETTE.target1, PALETTE.target0];
  // index 0 = bull → red, 1 = inner → blue, 2 = mid → black, 3 = outer → white
  return palette[i];
}

function drawHits(ctx, s) {
  for (const h of s.hits) {
    if (!h.ring) continue;        // miss - skip
    ctx.fillStyle = PALETTE.arrowHead;
    ctx.fillRect((h.x | 0) - 2, (h.y | 0) - 2, 4, 4);
    ctx.fillStyle = PALETTE.arrow;
    ctx.fillRect((h.x | 0) - 6, (h.y | 0) - 1, 4, 2);
    ctx.fillStyle = PALETTE.fletch;
    ctx.fillRect((h.x | 0) - 8, (h.y | 0) - 2, 2, 4);
  }
}

function drawBow(ctx, s) {
  // Body.
  ctx.fillStyle = PALETTE.bow;
  ctx.fillRect(BOW_X - 1, BOW_Y - 18, 3, 36);
  ctx.fillStyle = PALETTE.bowHi;
  ctx.fillRect(BOW_X - 1, BOW_Y - 18, 1, 36);
  // Curved tips (rough).
  ctx.fillRect(BOW_X - 4, BOW_Y - 18, 4, 4);
  ctx.fillRect(BOW_X - 4, BOW_Y + 14, 4, 4);
  ctx.fillRect(BOW_X + 2, BOW_Y - 18, 2, 2);
  ctx.fillRect(BOW_X + 2, BOW_Y + 16, 2, 2);
  // String — straight by default, pulled when aiming.
  ctx.strokeStyle = PALETTE.string;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(BOW_X + 0.5, BOW_Y - 18);
  if (s.aim) {
    // Pull anchor toward the drag direction.
    const dx = s.aim.x - BOW_X, dy = s.aim.y - BOW_Y;
    const len = Math.hypot(dx, dy);
    const k = Math.min(20, len);
    const ux = dx / (len || 1), uy = dy / (len || 1);
    ctx.lineTo(BOW_X + ux * k, BOW_Y + uy * k);
  }
  ctx.lineTo(BOW_X + 0.5, BOW_Y + 18);
  ctx.stroke();
}

function drawArrow(ctx, s) {
  if (!s.arrow) return;
  const a = s.arrow;
  const ang = Math.atan2(a.vy, a.vx);
  ctx.save();
  ctx.translate(a.x, a.y);
  ctx.rotate(ang);
  // Shaft.
  ctx.fillStyle = PALETTE.arrow;
  ctx.fillRect(-10, -1, 16, 2);
  // Head.
  ctx.fillStyle = PALETTE.arrowHead;
  ctx.fillRect(6, -2, 4, 4);
  // Fletch.
  ctx.fillStyle = PALETTE.fletch;
  ctx.fillRect(-12, -2, 3, 4);
  ctx.restore();
}

function drawAimGuide(ctx, s) {
  if (!s.aim) return;
  const dx = s.aim.x - BOW_X, dy = s.aim.y - BOW_Y;
  const len = Math.hypot(dx, dy);
  if (len < 10) return;
  const ux = dx / len, uy = dy / len;
  // Drag trail (yellow dots behind).
  ctx.fillStyle = PALETTE.aimGuide;
  for (let d = 12; d < Math.min(140, len); d += 8) {
    ctx.fillRect(((BOW_X + ux * d - 1) | 0), ((BOW_Y + uy * d - 1) | 0), 2, 2);
  }
  // Shot trajectory preview - a short opposite-direction guide.
  const fwdLen = Math.min(140, len) * 1.4;
  ctx.fillStyle = len > 120 ? PALETTE.aimMax : PALETTE.aimGuide;
  for (let d = 14; d < fwdLen; d += 10) {
    ctx.fillRect(((BOW_X - ux * d - 1) | 0), ((BOW_Y - uy * d - 1) | 0), 2, 2);
  }
  // Power dot at drag end.
  ctx.fillStyle = len > 120 ? PALETTE.aimMax : PALETTE.aimGuide;
  ctx.fillRect((BOW_X + ux * Math.min(140, len) - 3) | 0, (BOW_Y + uy * Math.min(140, len) - 3) | 0, 6, 6);
}

function drawWindIcon(ctx, s, lang) {
  const x = 290, y = 60;
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = '9px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'top';
  ctx.fillText(t(lang, 'wind'), x + 18, y - 12);
  const v = s.activeWind;
  if (Math.abs(v) < 1) {
    ctx.fillText('---', x + 18, y);
    return;
  }
  const mag = Math.min(Math.abs(v) / 80, 1);
  ctx.fillStyle = v > 0 ? '#a8d84a' : '#f2cf3f';
  if (v > 0) {
    ctx.fillRect(x, y, 24 * mag + 8, 2);
    ctx.fillRect(x + 24 * mag + 4, y - 2, 2, 6);
    ctx.fillRect(x + 24 * mag + 6, y - 1, 2, 4);
  } else {
    const w = 24 * mag + 8;
    ctx.fillRect(x + 24 - w, y, w, 2);
    ctx.fillRect(x + 24 - w - 2, y - 2, 2, 6);
    ctx.fillRect(x + 24 - w - 4, y - 1, 2, 4);
  }
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
  ctx.fillText(t(lang, 'arrows') + ' ' + (10 - s.quiver) + '/' + 10, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, 352, 16);
}

