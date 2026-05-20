// Pixel-art rendering for Pixel Joust. 360x480 world units.

const PALETTE = {
  sky:      '#1a142a',
  skyHi:    '#28203a',
  ground:   '#3a2a1a',
  groundHi: '#5c422a',
  groundLo: '#1c130a',
  platform: '#7a4a1f',
  platHi:   '#a06a3a',
  platLo:   '#4a2a0f',
  player:   '#ffe04a',           // ostrich body
  playerHi: '#fff0c0',
  playerLo: '#a07a14',
  beak:     '#ff7a3a',
  lance:    '#cfe8ff',
  lanceTip: '#f4d27b',
  enemy:    '#ff7a7a',
  enemyHi:  '#ff9b9b',
  enemyLo:  '#7a1e1e',
  egg:      '#f0d6a8',
  eggHi:    '#fff7ed',
  hud:      '#06061a',
  hudText:  '#f8f5e8',
  hudDim:   '#a0a8b8',
  heart:    '#ff4a5a',
  win:      '#5fc06e',
  ctrl:     '#28315c',
  ctrlHi:   '#3c4576',
  ctrlOn:   '#5fc0ff',
  ctrlText: '#f8f5e8',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.sky;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.skyHi;
  for (let i = 0; i < 30; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawArena(ctx) {
  // Ground floor band.
  ctx.fillStyle = PALETTE.groundLo;
  ctx.fillRect(0, ARENA_BOTTOM, VW, VH - ARENA_BOTTOM);
  ctx.fillStyle = PALETTE.ground;
  ctx.fillRect(0, ARENA_BOTTOM, VW, 4);
  ctx.fillStyle = PALETTE.groundHi;
  ctx.fillRect(0, ARENA_BOTTOM, VW, 1);
  // Platforms.
  for (const pl of PLATFORMS) drawPlatform(ctx, pl);
}

function drawPlatform(ctx, pl) {
  ctx.fillStyle = PALETTE.platLo;
  ctx.fillRect(pl.x - 1, pl.y - 1, pl.w + 2, 8);
  ctx.fillStyle = PALETTE.platform;
  ctx.fillRect(pl.x, pl.y, pl.w, 6);
  ctx.fillStyle = PALETTE.platHi;
  ctx.fillRect(pl.x, pl.y, pl.w, 2);
}

function drawRider(ctx, r, isPlayer) {
  if (!r.alive && isPlayer && r.respawn > 0) return;
  if (isPlayer && r.hitFlash > 0 && Math.floor(r.hitFlash * 12) % 2 === 0) return;
  const x = r.x, y = r.y;
  const dir = r.face >= 0 ? 1 : -1;
  // Lance — a thin diagonal stick pointing in face direction + slightly up.
  ctx.fillStyle = PALETTE.lance;
  for (let i = 0; i < 12; i++) {
    ctx.fillRect((x + dir * (4 + i)) | 0, (y - 6 - i * 0.3) | 0, 1, 1);
  }
  ctx.fillStyle = PALETTE.lanceTip;
  ctx.fillRect((x + dir * 16) | 0, (y - 10) | 0, 2, 2);
  // Ostrich body (or buzzard for enemy).
  const body = isPlayer ? PALETTE.player : PALETTE.enemy;
  const hi   = isPlayer ? PALETTE.playerHi : PALETTE.enemyHi;
  const lo   = isPlayer ? PALETTE.playerLo : PALETTE.enemyLo;
  ctx.fillStyle = lo;
  ctx.fillRect(x - 9, y - 8, 18, 14);
  ctx.fillStyle = body;
  ctx.fillRect(x - 8, y - 7, 16, 12);
  ctx.fillStyle = hi;
  ctx.fillRect(x - 8, y - 7, 16, 2);
  // Neck + head — facing dir.
  ctx.fillStyle = lo;
  ctx.fillRect(x + dir * 2 - 1, y - 12, 3, 8);
  ctx.fillStyle = body;
  ctx.fillRect(x + dir * 2, y - 12, 2, 7);
  // Beak
  ctx.fillStyle = PALETTE.beak;
  ctx.fillRect(x + dir * 4, y - 11, 3, 2);
  // Eye
  ctx.fillStyle = '#0a0a18';
  ctx.fillRect(x + dir * 3, y - 13, 1, 1);
  // Wings — flap = small bob (visual cue via vy < 0 for player).
  ctx.fillStyle = hi;
  const flap = (r.vy || 0) < -50 ? -4 : 0;
  ctx.fillRect(x - 10, y - 4 + flap, 4, 3);
  ctx.fillRect(x + 6,  y - 4 + flap, 4, 3);
}

function drawEgg(ctx, e) {
  const blink = e.timer < 1.5 && Math.floor(e.timer * 8) % 2 === 0;
  ctx.fillStyle = blink ? '#ff7a7a' : PALETTE.egg;
  ctx.fillRect(e.x - 4, e.y - 5, 8, 10);
  ctx.fillStyle = PALETTE.eggHi;
  ctx.fillRect(e.x - 3, e.y - 4, 2, 3);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) drawHeart(ctx, 130 + i * 12, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 16);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

function controlRects() {
  const y = VH - 56, w = 84, h = 48, gap = 8;
  const total = w * 3 + gap * 2;
  const x0 = ((VW - total) / 2) | 0;
  return {
    left:  { x: x0,                     y, w, h, label: '←' },
    flap:  { x: x0 + (w + gap),         y, w, h, label: '✦' },
    right: { x: x0 + (w + gap) * 2,     y, w, h, label: '→' },
  };
}

function drawControls(ctx, input) {
  const rs = controlRects();
  for (const key of Object.keys(rs)) {
    const r = rs[key];
    const on = (key !== 'flap' && input && input[key]);
    ctx.fillStyle = on ? PALETTE.ctrlOn : PALETTE.ctrl;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = PALETTE.ctrlHi;
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = PALETTE.ctrlText;
    ctx.font = 'bold 22px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(r.label, r.x + r.w / 2, r.y + r.h / 2);
    ctx.textBaseline = 'alphabetic';
  }
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.55);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.player && !s.player.alive ? `rgba(255,80,80,${0.5 * a})` :
                                                `rgba(255,255,255,${0.18 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
