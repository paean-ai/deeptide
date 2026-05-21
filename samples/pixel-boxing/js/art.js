// Pixel-art rendering for Pixel Boxing. 360x480 world units.

const PALETTE = {
  bg:        '#1a1020',
  ring:      '#3a2c4a',
  ringHi:    '#5a4670',
  rope:      '#e85a3a',
  ropeHi:    '#ff8a6a',
  mat:       '#d8c4a0',
  matHi:     '#f0dcb8',
  player:    '#5fc0ff',
  playerHi:  '#a8e0ff',
  playerLo:  '#205a8a',
  glove:     '#e85a3a',
  gloveHi:   '#ff8a6a',
  tell:      '#ffe04a',
  tellGlow:  'rgba(255,224,74,0.45)',
  hpGood:    '#5fc06e',
  hpLow:     '#ff5a5a',
  hpBack:    '#0e0a14',
  hud:       '#0e0a14',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  msg:       '#ffe04a',
  ctrl:      '#28315c',
  ctrlHi:    '#3c4576',
  ctrlText:  '#f8f5e8',
  win:       '#5fc06e',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // Ring: a trapezoid mat with rope lines, drawn in pseudo-perspective.
  ctx.fillStyle = PALETTE.ring;
  ctx.fillRect(0, 60, VW, 280);
  // Mat.
  ctx.fillStyle = PALETTE.mat;
  ctx.beginPath();
  ctx.moveTo(40, 300); ctx.lineTo(320, 300);
  ctx.lineTo(360, 110); ctx.lineTo(0, 110);
  ctx.closePath(); ctx.fill();
  ctx.fillStyle = PALETTE.matHi;
  ctx.fillRect(0, 110, VW, 3);
  // Three rope lines across the back.
  for (let i = 0; i < 3; i++) {
    const y = 78 + i * 14;
    ctx.fillStyle = i % 2 ? PALETTE.ropeHi : PALETTE.rope;
    ctx.fillRect(0, y, VW, 4);
  }
  // Corner posts.
  ctx.fillStyle = PALETTE.ringHi;
  ctx.fillRect(0, 72, 8, 50);
  ctx.fillRect(VW - 8, 72, 8, 50);
}

// The opponent stands at the top-centre facing the player.
function drawFoe(ctx, s) {
  const f = s.foe;
  const cx = VW / 2, cy = 196;
  const flash = f.hitFlash > 0 && Math.floor(f.hitFlash * 20) % 2 === 0;
  const stagger = f.state === 'stagger';
  const lean = stagger ? Math.sin(performance.now() / 60) * 4 : 0;
  ctx.save();
  ctx.translate(cx + lean, cy);
  // Body.
  ctx.fillStyle = flash ? '#ffffff' : s.cfg.color;
  ctx.fillRect(-22, -36, 44, 56);
  ctx.fillStyle = flash ? '#ffffff' : shade(s.cfg.color, 30);
  ctx.fillRect(-22, -36, 44, 4);
  // Head.
  ctx.fillStyle = flash ? '#ffffff' : '#e8b890';
  ctx.fillRect(-12, -58, 24, 22);
  // Eyes — X'd out when staggered.
  ctx.fillStyle = '#1a1020';
  if (stagger) {
    ctx.fillText('><', 0, 0);     // (fallback; pixel X drawn below)
    ctx.fillRect(-9, -50, 6, 2); ctx.fillRect(-9, -46, 6, 2);
    ctx.fillRect(3, -50, 6, 2);  ctx.fillRect(3, -46, 6, 2);
  } else {
    ctx.fillRect(-8, -50, 5, 4);
    ctx.fillRect(3, -50, 5, 4);
  }
  // Gloves — extend toward the strike side during windup / strike.
  let lx = -30, rx = 22, gy = -10;
  if ((f.state === 'windup' || f.state === 'strike') && f.tell) {
    const ext = f.state === 'strike' ? 26 : 12;
    if (f.tell === 'L') lx -= ext; else rx += ext;
    gy = f.state === 'strike' ? 14 : 0;
  }
  // Tell glow on the winding-up arm.
  if (f.state === 'windup' && f.tell) {
    ctx.fillStyle = PALETTE.tellGlow;
    const tx = f.tell === 'L' ? lx : rx;
    ctx.fillRect(tx - 6, gy - 6, 20, 20);
  }
  ctx.fillStyle = (f.state === 'windup' && f.tell) ? PALETTE.tell : PALETTE.glove;
  ctx.fillRect(lx, gy, 12, 12);
  ctx.fillRect(rx - 4, gy, 12, 12);
  ctx.restore();
}

// The player is shown from behind near the bottom.
function drawPlayer(ctx, s) {
  const p = s.player;
  let cx = VW / 2;
  if (p.pose === 'dodgeL') cx -= 40;
  else if (p.pose === 'dodgeR') cx += 40;
  const cy = 372;
  const flash = p.hitFlash > 0 && Math.floor(p.hitFlash * 20) % 2 === 0;
  ctx.save();
  ctx.translate(cx, cy);
  // Shoulders / back.
  ctx.fillStyle = flash ? '#ffffff' : PALETTE.player;
  ctx.fillRect(-26, -10, 52, 40);
  ctx.fillStyle = flash ? '#ffffff' : PALETTE.playerHi;
  ctx.fillRect(-26, -10, 52, 4);
  // Head (back of).
  ctx.fillStyle = flash ? '#ffffff' : '#3a2a1a';
  ctx.fillRect(-12, -30, 24, 22);
  // Gloves — punch poses thrust a glove forward (upward on screen).
  ctx.fillStyle = PALETTE.glove;
  let lgy = -2, rgy = -2;
  if (p.pose === 'punchL') lgy = -30;
  if (p.pose === 'punchR') rgy = -30;
  if (p.pose === 'block') { lgy = -16; rgy = -16; }
  ctx.fillRect(-34, lgy, 14, 14);
  ctx.fillRect(20, rgy, 14, 14);
  ctx.fillStyle = PALETTE.gloveHi;
  ctx.fillRect(-34, lgy, 14, 3);
  ctx.fillRect(20, rgy, 14, 3);
  ctx.restore();
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 56);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 12);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 12);
  // Two HP bars.
  drawBar(ctx, 8,  24, VW - 16, 10, s.foe.hp / s.foe.maxhp, 'FOE');
  drawBar(ctx, 8,  40, VW - 16, 10, s.player.hp / s.player.maxhp, 'YOU');
  // Floating message.
  if (s.msgT > 0) {
    ctx.globalAlpha = Math.min(1, s.msgT / 0.4);
    ctx.fillStyle = PALETTE.msg;
    ctx.font = 'bold 20px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(s.msg, VW / 2, 260);
    ctx.globalAlpha = 1;
  }
}

function drawBar(ctx, x, y, w, h, frac, label) {
  ctx.fillStyle = PALETTE.hpBack;
  ctx.fillRect(x, y, w, h);
  const f = Math.max(0, Math.min(1, frac));
  ctx.fillStyle = f > 0.35 ? PALETTE.hpGood : PALETTE.hpLow;
  ctx.fillRect(x + 1, y + 1, (w - 2) * f, h - 2);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.font = 'bold 8px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, x + 3, y + h / 2);
}

// Four control buttons in a 2x2 grid at the bottom.
function controlRects() {
  const w = 168, h = 40, gap = 8;
  const x0 = (VW - w * 2 - gap) / 2;
  const y0 = VH - h * 2 - gap - 8;
  return {
    dodgeL: { x: x0,             y: y0,            w, h },
    dodgeR: { x: x0 + w + gap,   y: y0,            w, h },
    block:  { x: x0,             y: y0 + h + gap,  w, h },
    punch:  { x: x0 + w + gap,   y: y0 + h + gap,  w, h },
  };
}

function drawControls(ctx, lang) {
  const rs = controlRects();
  const labels = { dodgeL: t(lang,'dodgeL'), dodgeR: t(lang,'dodgeR'),
                   block: t(lang,'block'), punch: t(lang,'punch') };
  for (const key of Object.keys(rs)) {
    const r = rs[key];
    ctx.fillStyle = key === 'punch' ? '#e85a3a' : PALETTE.ctrl;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = key === 'punch' ? '#ff8a6a' : PALETTE.ctrlHi;
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = PALETTE.ctrlText;
    ctx.font = 'bold 13px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(labels[key], r.x + r.w / 2, r.y + r.h / 2);
    ctx.textBaseline = 'alphabetic';
  }
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.35);
  ctx.fillStyle = s.won ? `rgba(95,192,110,${0.3 * a})` :
                  s.player.hitFlash > 0 ? `rgba(255,80,80,${0.4 * a})` :
                                          `rgba(255,255,255,${0.16 * a})`;
  ctx.fillRect(0, 56, VW, VH - 56);
}

function shade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.min(255, ((n >> 16) & 255) + amt);
  const g = Math.min(255, ((n >> 8) & 255) + amt);
  const b = Math.min(255, (n & 255) + amt);
  return '#' + [r, g, b].map(v => v.toString(16).padStart(2, '0')).join('');
}
