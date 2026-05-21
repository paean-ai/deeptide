// Pixel Aegis - all canvas drawing: the circular arena, shield, title.

const PAL = {
  bg0: '#0c1020', bg1: '#1a1734', ink: '#070610',
  panel: '#241f3c', panelHi: '#362f54', text: '#f3f1e6', dim: '#8f88a8',
  core: '#6fd0e8', coreDk: '#2f7e96', coreHi: '#cdf2ff',
  shield: '#ffe27a', shieldHi: '#fff6cf', pulse: '#8fe0ff',
  shot: '#ff6b6b', shotHi: '#ffc0c0',
  gunner: '#9aa0b4', twin: '#e8554f', burst: '#f2a83e',
  good: '#7bd88f', bad: '#ff6470', star: '#ffe27a', heart: '#ff5d6c', heartDk: '#46232f',
};

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}
function disc(ctx, x, y, r, c) {
  ctx.fillStyle = c; ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.fill();
}
function pointAt(a, r) { return { x: CORE_X + Math.cos(a) * r, y: CORE_Y + Math.sin(a) * r }; }

const PULSE_BTN = { x: 110, y: 426, w: 140, h: 42 };

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

// ---- arena ---------------------------------------------------------------
function drawArena(ctx, s, now) {
  drawBackdrop(ctx);
  // perimeter ring
  ctx.strokeStyle = PAL.panel; ctx.lineWidth = 2;
  ctx.beginPath(); ctx.arc(CORE_X, CORE_Y, PERIM_R, 0, Math.PI * 2); ctx.stroke();

  // projectiles
  for (const p of s.projectiles) {
    const q = pointAt(p.angle, p.r);
    disc(ctx, q.x, q.y, 5, PAL.shot);
    disc(ctx, q.x, q.y, 2, PAL.shotHi);
  }

  // shield arc (full ring while the pulse is active)
  const pulsing = s.pulseT > 0;
  ctx.lineCap = 'round';
  ctx.lineWidth = 9;
  if (pulsing) {
    ctx.strokeStyle = PAL.pulse;
    ctx.beginPath(); ctx.arc(CORE_X, CORE_Y, SHIELD_R, 0, Math.PI * 2); ctx.stroke();
  } else {
    ctx.strokeStyle = PAL.shield;
    ctx.beginPath();
    ctx.arc(CORE_X, CORE_Y, SHIELD_R, s.shieldAngle - SHIELD_HALF, s.shieldAngle + SHIELD_HALF);
    ctx.stroke();
    ctx.strokeStyle = PAL.shieldHi; ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(CORE_X, CORE_Y, SHIELD_R, s.shieldAngle - SHIELD_HALF, s.shieldAngle + SHIELD_HALF);
    ctx.stroke();
  }
  ctx.lineWidth = 1; ctx.lineCap = 'butt';

  // shooters
  for (const sh of s.shooters) {
    const q = pointAt(sh.angle, PERIM_R);
    const col = PAL[sh.type];
    px(ctx, q.x - 9, q.y - 9, 18, 18, PAL.ink);
    px(ctx, q.x - 7, q.y - 7, 14, 14, col);
    px(ctx, q.x - 7, q.y - 7, 14, 3, '#ffffff66');
    // barrel toward the core
    const inw = pointAt(sh.angle, PERIM_R - 13);
    px(ctx, inw.x - 3, inw.y - 3, 6, 6, col);
    // hp pips
    const mx = TYPE_HP[sh.type];
    for (let i = 0; i < mx; i++) {
      px(ctx, q.x - mx * 3 + i * 6, q.y + 11, 4, 3, i < sh.hp ? PAL.good : PAL.ink);
    }
  }

  // fx
  for (const f of s.fx) {
    const r = f.kind === 'kill' ? PERIM_R : (f.kind === 'hurt' ? CORE_R + 6 : SHIELD_R);
    const q = pointAt(f.angle, r);
    const a = Math.max(0, f.t / 0.4);
    ctx.globalAlpha = a;
    const c = f.kind === 'hurt' ? PAL.bad : (f.kind === 'kill' ? PAL.star : PAL.shieldHi);
    disc(ctx, q.x, q.y, 4 + (1 - a) * 9, c);
    ctx.globalAlpha = 1;
  }

  // core
  const hurt = s.fx.some(f => f.kind === 'hurt');
  disc(ctx, CORE_X, CORE_Y, CORE_R + 3, PAL.ink);
  disc(ctx, CORE_X, CORE_Y, CORE_R, hurt ? PAL.bad : PAL.coreDk);
  disc(ctx, CORE_X, CORE_Y, CORE_R - 6, PAL.core);
  disc(ctx, CORE_X, CORE_Y, CORE_R - 12, PAL.coreHi);
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  fillText(ctx, L(s.stage.name).toUpperCase(), 180, 22, 16, PAL.text);
  for (let i = 0; i < CORE_HP; i++) {
    const on = i < s.coreHp;
    const x = 180 - CORE_HP * 9 + i * 18;
    px(ctx, x, 38, 13, 7, on ? PAL.heart : PAL.heartDk);
    px(ctx, x + 2, 35, 9, 3, on ? PAL.heart : PAL.heartDk);
  }
  fillText(ctx, s.shooters.length + ' LEFT', 320, 22, 10, PAL.dim, 'right');
}

function drawPulseBtn(ctx, s) {
  const r = PULSE_BTN, ready = s.pulseCd <= 0 && !s.over;
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, ready ? PAL.pulse : PAL.panel);
  if (ready) {
    fillText(ctx, '◎ ' + t('pulse'), r.x + r.w / 2, r.y + r.h / 2, 15, PAL.ink);
  } else {
    fillText(ctx, t('pulse') + '  ' + s.pulseCd.toFixed(1) + 's', r.x + r.w / 2, r.y + r.h / 2, 12, PAL.dim);
  }
}

// ---- buttons + stars -----------------------------------------------------
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 44 ? 16 : 13, active ? PAL.ink : color);
}
function drawStars(ctx, cx, cy, n, size) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * (size + 6), on = i < n;
    px(ctx, x - size / 2, cy - size / 2, size, size, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy - size / 2 - 3, 6, 3, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy + size / 2, 6, 3, on ? PAL.star : PAL.panel);
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  const cx = 180, cy = 196;
  ctx.strokeStyle = PAL.panel; ctx.lineWidth = 2;
  ctx.beginPath(); ctx.arc(cx, cy, 96, 0, Math.PI * 2); ctx.stroke();
  for (let i = 0; i < 6; i++) {
    const a = now / 1400 + i * Math.PI / 3;
    const x = cx + Math.cos(a) * 96, y = cy + Math.sin(a) * 96;
    px(ctx, x - 6, y - 6, 12, 12, PAL.gunner);
  }
  ctx.strokeStyle = PAL.shield; ctx.lineWidth = 8; ctx.lineCap = 'round';
  const sa = -now / 700;
  ctx.beginPath(); ctx.arc(cx, cy, 54, sa - 0.7, sa + 0.7); ctx.stroke();
  ctx.lineWidth = 1; ctx.lineCap = 'butt';
  disc(ctx, cx, cy, 18, PAL.coreDk);
  disc(ctx, cx, cy, 11, PAL.core);
  disc(ctx, cx, cy, 5, PAL.coreHi);
}
