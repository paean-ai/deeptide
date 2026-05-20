// Pixel-art rendering for Pixel Sumo. 360x480 world units.

const PALETTE = {
  bg:        '#1d2240',
  outside:   '#0d1228',
  ringEdge:  '#a36835',
  ring:      '#c79b5f',
  ringMark:  '#7d4f29',
  centerLine:'#7d4f29',
  player:    '#e8554f',
  playerDark:'#a8373a',
  playerHi:  '#fff',
  ai:        '#4a9be8',
  aiDark:    '#1f5494',
  beltP:     '#9aa6cc',
  beltA:     '#dde6ff',
  aimGuide:  '#f7e69a',
  aimMax:    '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function drawScene(ctx, s, lang) {
  ctx.fillStyle = PALETTE.outside;
  ctx.fillRect(0, 0, 360, 480);
  drawRing(ctx);
  drawWrestler(ctx, s.player, 'p');
  drawWrestler(ctx, s.ai, 'a');
  drawAimGuide(ctx, s);
  drawHud(ctx, s, lang);
}

function drawRing(ctx) {
  ctx.fillStyle = PALETTE.ringEdge;
  ctx.beginPath(); ctx.arc(RING_CX, RING_CY, RING_R + 6, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.ring;
  ctx.beginPath(); ctx.arc(RING_CX, RING_CY, RING_R, 0, Math.PI * 2); ctx.fill();
  // Centre cross.
  ctx.strokeStyle = PALETTE.centerLine;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(RING_CX - 12, RING_CY); ctx.lineTo(RING_CX + 12, RING_CY);
  ctx.moveTo(RING_CX, RING_CY - 12); ctx.lineTo(RING_CX, RING_CY + 12);
  ctx.stroke();
  // Sand texture - light specks.
  ctx.fillStyle = PALETTE.ringMark;
  for (let i = 0; i < 30; i++) {
    const a = (i * 0.81) % (Math.PI * 2);
    const r = RING_R * (0.2 + (i * 0.07) % 0.7);
    ctx.fillRect((RING_CX + Math.cos(a) * r) | 0, (RING_CY + Math.sin(a) * r) | 0, 1, 1);
  }
}

function drawWrestler(ctx, w, who) {
  if (!w.alive) {
    // Tumbled wrestler off-ring: just a flat dim oval.
    ctx.fillStyle = who === 'p' ? PALETTE.playerDark : PALETTE.aiDark;
    ctx.beginPath(); ctx.ellipse(w.x, w.y, WRESTLER_R, WRESTLER_R / 2, 0, 0, Math.PI * 2); ctx.fill();
    return;
  }
  ctx.fillStyle = 'rgba(0,0,0,0.3)';
  ctx.beginPath(); ctx.arc(w.x + 1, w.y + 3, WRESTLER_R, 0, Math.PI * 2); ctx.fill();
  const dark = who === 'p' ? PALETTE.playerDark : PALETTE.aiDark;
  const base = who === 'p' ? PALETTE.player    : PALETTE.ai;
  ctx.fillStyle = dark;
  ctx.beginPath(); ctx.arc(w.x, w.y, WRESTLER_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = base;
  ctx.beginPath(); ctx.arc(w.x, w.y, WRESTLER_R - 1, 0, Math.PI * 2); ctx.fill();
  // Belt (mawashi).
  ctx.fillStyle = who === 'p' ? PALETTE.beltP : PALETTE.beltA;
  ctx.fillRect((w.x - WRESTLER_R + 1) | 0, (w.y + WRESTLER_R / 2) | 0, WRESTLER_R * 2 - 2, 3);
  // Topknot / eye dot.
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect((w.x - 2) | 0, (w.y - WRESTLER_R + 2) | 0, 4, 2);
  ctx.fillStyle = '#0c1230';
  ctx.fillRect((w.x - 4) | 0, (w.y - 2) | 0, 2, 2);
  ctx.fillRect((w.x + 2) | 0, (w.y - 2) | 0, 2, 2);
}

function drawAimGuide(ctx, s) {
  if (!s.aim || !s.player.alive) return;
  const dx = s.aim.x - s.player.x, dy = s.aim.y - s.player.y;
  const len = Math.hypot(dx, dy);
  if (len < 8) return;
  const ux = dx / len, uy = dy / len;
  ctx.fillStyle = PALETTE.aimGuide;
  for (let d = 12; d < Math.min(120, len); d += 8) {
    ctx.fillRect((s.player.x + ux * d - 1) | 0, (s.player.y + uy * d - 1) | 0, 2, 2);
  }
  // Forward (opposite the drag) preview.
  ctx.fillStyle = len > 110 ? PALETTE.aimMax : PALETTE.aimGuide;
  const fwd = Math.min(120, len) * 1.4;
  for (let d = 14; d < fwd; d += 10) {
    ctx.fillRect((s.player.x - ux * d - 1) | 0, (s.player.y - uy * d - 1) | 0, 2, 2);
  }
  ctx.fillRect((s.player.x + ux * Math.min(120, len) - 3) | 0, (s.player.y + uy * Math.min(120, len) - 3) | 0, 6, 6);
}

function drawHud(ctx, s, lang) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  // Score markers (one wrestler each — show name/colour as VS dot).
  ctx.textAlign = 'center';
  ctx.fillStyle = PALETTE.player;
  ctx.fillText('●', 150, 16);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText('VS', 180, 16);
  ctx.fillStyle = PALETTE.ai;
  ctx.fillText('●', 210, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  const ms = (s.elapsed * 10) | 0;
  ctx.fillText((ms / 10).toFixed(1) + 's', 352, 16);
}
