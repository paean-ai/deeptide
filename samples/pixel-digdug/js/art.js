// Pixel-art rendering for Pixel DigDug. 360x480 world units.

const PALETTE = {
  sky:      '#0a0a1c',
  skyHi:    '#101630',
  dirt:     '#7a4a1f',
  dirtHi:   '#a06a3a',
  dirtLo:   '#4a2a0f',
  dirtSpec: '#5a3614',
  tunnel:   '#241612',
  tunnelHi: '#3a221a',
  rock:     '#7a8088',
  rockHi:   '#a8b0b8',
  rockLo:   '#3a4048',
  player:   '#ffe04a',
  playerHi: '#fff0c0',
  playerLo: '#a07a14',
  playerEye:'#0a0a0a',
  hose:     '#a0a8b8',
  hud:      '#06061a',
  hudText:  '#f8f5e8',
  hudDim:   '#a0a8b8',
  heart:    '#ff4a5a',
  win:      '#5fc06e',
  bad:      '#ff5a3a',
  pumpFill: '#5fc06e',
  pumpHold: '#ffd34a',
  ctrl:     '#28315c',
  ctrlHi:   '#3c4576',
  ctrlOn:   '#5fc0ff',
  ctrlText: '#f8f5e8',
};

function cellRect(c, r) {
  return { x: BOARD_OX + c * CELL, y: BOARD_OY + r * CELL, w: CELL, h: CELL };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.sky;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.skyHi;
  for (let i = 0; i < 22; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s) {
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    const t = s.tiles[r * COLS + c];
    const rect = cellRect(c, r);
    if (r < SKY_ROWS) {
      ctx.fillStyle = PALETTE.sky;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      continue;
    }
    if (t === 0) {
      ctx.fillStyle = PALETTE.dirt;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.fillStyle = PALETTE.dirtHi;
      ctx.fillRect(rect.x, rect.y, rect.w, 2);
      ctx.fillStyle = PALETTE.dirtLo;
      ctx.fillRect(rect.x, rect.y + rect.h - 2, rect.w, 2);
      ctx.fillStyle = PALETTE.dirtSpec;
      ctx.fillRect(rect.x + 4, rect.y + 6, 2, 2);
      ctx.fillRect(rect.x + 12, rect.y + 12, 2, 2);
    } else if (t === 1) {
      ctx.fillStyle = PALETTE.tunnel;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.fillStyle = PALETTE.tunnelHi;
      ctx.fillRect(rect.x, rect.y, rect.w, 1);
    } else if (t === 2) {
      // Rock cell. Will be replaced by drawRocks for falling ones.
    }
  }
  for (const rk of s.rocks) {
    if (rk.dead) continue;
    drawRock(ctx, rk);
  }
}

function drawRock(ctx, rk) {
  const rect = cellRect(rk.c, Math.floor(rk.falling ? (rk.y == null ? rk.r : rk.y) : rk.r));
  ctx.fillStyle = PALETTE.rockLo;
  ctx.fillRect(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4);
  ctx.fillStyle = PALETTE.rock;
  ctx.fillRect(rect.x + 3, rect.y + 3, rect.w - 6, rect.h - 6);
  ctx.fillStyle = PALETTE.rockHi;
  ctx.fillRect(rect.x + 3, rect.y + 3, rect.w - 6, 2);
}

function drawPlayer(ctx, p) {
  if (!p.alive) return;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return;
  const rect = cellRect(p.c, p.r);
  const cx = rect.x + rect.w / 2, cy = rect.y + rect.h / 2;
  ctx.fillStyle = PALETTE.playerLo;
  ctx.fillRect(cx - 8, cy - 7, 16, 14);
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(cx - 7, cy - 6, 14, 12);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(cx - 7, cy - 6, 14, 2);
  // Helmet stripe
  ctx.fillStyle = '#0a1024';
  ctx.fillRect(cx - 7, cy - 4, 14, 2);
  // Eyes (face direction)
  ctx.fillStyle = '#fff7ed';
  const dx = p.face === 'left' ? -1 : p.face === 'right' ? 1 : 0;
  const dy = p.face === 'up' ? -1 : p.face === 'down' ? 1 : 0;
  ctx.fillRect(cx - 5 + dx, cy + dy, 2, 2);
  ctx.fillRect(cx + 3 + dx, cy + dy, 2, 2);
}

function drawEnemies(ctx, s) {
  for (const e of s.enemies) {
    if (!e.alive) continue;
    const rect = cellRect(e.c, e.r);
    const cx = rect.x + rect.w / 2, cy = rect.y + rect.h / 2;
    // Inflated size scales with pumped count.
    const scale = 1 + e.pumped * 0.25;
    const r = (rect.w * 0.36) * scale;
    // Shadow + body
    ctx.fillStyle = '#0a0a18';
    fillDisk(ctx, cx, cy + 1, r);
    ctx.fillStyle = e.color;
    fillDisk(ctx, cx, cy, r);
    ctx.fillStyle = '#fff7ed';
    fillDisk(ctx, cx - 2, cy - 2, r * 0.45);
    // Eyes
    ctx.fillStyle = '#0a0a18';
    ctx.fillRect(cx - 3, cy - 2, 2, 2);
    ctx.fillRect(cx + 1, cy - 2, 2, 2);
    // Inflation gauge: small dot triplet below the enemy showing pumps.
    for (let i = 0; i < POP_PUMPS; i++) {
      ctx.fillStyle = i < e.pumped ? PALETTE.pumpFill : PALETTE.tunnelHi;
      ctx.fillRect(cx - 4 + i * 4, cy + r + 2, 3, 2);
    }
  }
  // Pump hose: a dashed line from player toward the pump target.
  if (s.pumpTargetIdx >= 0) {
    const e = s.enemies[s.pumpTargetIdx];
    if (e && e.alive) {
      const p = s.player;
      const a = cellRect(p.c, p.r);
      const b = cellRect(e.c, e.r);
      const ax = a.x + a.w / 2, ay = a.y + a.h / 2;
      const bx = b.x + b.w / 2, by = b.y + b.h / 2;
      ctx.strokeStyle = PALETTE.hose;
      ctx.lineWidth = 2;
      ctx.setLineDash([3, 3]);
      ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
      ctx.setLineDash([]);
    }
  }
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
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
  const y = VH - 56, w = 56, h = 48, gap = 4;
  const total = w * 5 + gap * 4;
  const x0 = ((VW - total) / 2) | 0;
  return {
    left:  { x: x0,                       y, w, h, label: '←' },
    down:  { x: x0 + (w + gap),           y, w, h, label: '↓' },
    up:    { x: x0 + (w + gap) * 2,       y, w, h, label: '↑' },
    right: { x: x0 + (w + gap) * 3,       y, w, h, label: '→' },
    pump:  { x: x0 + (w + gap) * 4,       y, w, h, label: '★' },
  };
}

function drawControls(ctx, input, pumpHold) {
  const rs = controlRects();
  for (const key of Object.keys(rs)) {
    const r = rs[key];
    const dir = (key !== 'pump') && (input && input === key);
    const pump = (key === 'pump') && pumpHold;
    ctx.fillStyle = (dir || pump) ? PALETTE.ctrlOn : PALETTE.ctrl;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = PALETTE.ctrlHi;
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = PALETTE.ctrlText;
    ctx.font = 'bold 18px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(r.label, r.x + r.w / 2, r.y + r.h / 2);
    ctx.textBaseline = 'alphabetic';
  }
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.player && !s.player.alive ? `rgba(255,80,80,${0.5 * a})` :
                                                `rgba(255,255,255,${0.18 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
