// Pixel-art rendering for Pixel Snowboard. 360x480 world units.

const PALETTE = {
  snow:     '#e8eef8',
  snowHi:   '#ffffff',
  snowLo:   '#c8d4e6',
  tree:     '#2e6e3a',
  treeHi:   '#4a9a52',
  treeLo:   '#1a4424',
  trunk:    '#5a3a1e',
  rock:     '#8a8e98',
  rockHi:   '#b6bac4',
  rockLo:   '#54585f',
  gate:     '#e8554f',
  gateAlt:  '#4a9be8',
  ramp:     '#9ab0d0',
  rampHi:   '#c4d4e8',
  rider:    '#e8554f',
  riderHi:  '#ff8a6a',
  riderLo:  '#7a1e0c',
  board:    '#ffd34a',
  trail:    'rgba(120,140,180,0.4)',
  hud:      '#1c2438',
  hudText:  '#f8f5e8',
  hudDim:   '#a0a8b8',
  heart:    '#ff4a5a',
  win:      '#5fc06e',
};

function drawBackdrop(ctx, s) {
  // Snow field.
  ctx.fillStyle = PALETTE.snow;
  ctx.fillRect(0, 0, VW, VH);
  // Subtle moguls — stable speckle scrolling with the world.
  ctx.fillStyle = PALETTE.snowLo;
  const off = (s ? s.worldY : 0) % 40;
  for (let i = 0; i < 40; i++) {
    const sx = (i * 67 + 13) % VW;
    const sy = ((i * 53 + 7) % VH + off) % VH;
    ctx.fillRect(sx, sy, 3, 2);
  }
  ctx.fillStyle = PALETTE.snowHi;
  for (let i = 0; i < 20; i++) {
    const sx = (i * 97 + 31) % VW;
    const sy = ((i * 71 + 17) % VH + off) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawObstacles(ctx, s) {
  for (const o of s.obstacles) {
    const y = RIDER_Y + (o.y - s.worldY);
    if (y < -40 || y > VH + 40) continue;
    if (o.kind === 'tree') drawTree(ctx, o.x, y);
    else if (o.kind === 'rock') drawRock(ctx, o.x, y);
    else if (o.kind === 'gate') drawGate(ctx, o.x, y, o.w, o.scored);
    else if (o.kind === 'ramp') drawRamp(ctx, o.x, y, o.used);
  }
}

function drawTree(ctx, x, y) {
  // Trunk.
  ctx.fillStyle = PALETTE.trunk;
  ctx.fillRect((x - 2) | 0, (y + 6) | 0, 4, 8);
  // Three foliage tiers.
  for (let i = 0; i < 3; i++) {
    const w = 18 - i * 5;
    const ty = y - 14 + i * 9;
    ctx.fillStyle = PALETTE.treeLo;
    ctx.fillRect((x - w / 2) | 0, ty | 0, w, 10);
    ctx.fillStyle = PALETTE.tree;
    ctx.fillRect((x - w / 2 + 1) | 0, ty | 0, w - 2, 8);
    ctx.fillStyle = PALETTE.treeHi;
    ctx.fillRect((x - w / 2 + 1) | 0, ty | 0, w - 2, 2);
  }
}

function drawRock(ctx, x, y) {
  ctx.fillStyle = PALETTE.rockLo;
  ctx.fillRect((x - 12) | 0, (y - 6) | 0, 24, 14);
  ctx.fillStyle = PALETTE.rock;
  ctx.fillRect((x - 11) | 0, (y - 7) | 0, 22, 12);
  ctx.fillStyle = PALETTE.rockHi;
  ctx.fillRect((x - 11) | 0, (y - 7) | 0, 22, 3);
}

function drawGate(ctx, x, y, w, scored) {
  const col = scored ? '#8a8e98' : PALETTE.gate;
  const colB = scored ? '#8a8e98' : PALETTE.gateAlt;
  // Two flags, lane between them.
  drawFlag(ctx, x - w / 2, y, col);
  drawFlag(ctx, x + w / 2, y, colB);
}
function drawFlag(ctx, x, y, color) {
  ctx.fillStyle = '#3a3a3a';
  ctx.fillRect((x - 1) | 0, (y - 16) | 0, 2, 28);
  ctx.fillStyle = color;
  ctx.fillRect((x + 1) | 0, (y - 16) | 0, 12, 9);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect((x + 1) | 0, (y - 16) | 0, 12, 2);
}

function drawRamp(ctx, x, y, used) {
  ctx.fillStyle = used ? '#7a8694' : PALETTE.ramp;
  ctx.beginPath();
  ctx.moveTo(x - 16, y + 8);
  ctx.lineTo(x + 16, y + 8);
  ctx.lineTo(x + 16, y - 10);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = used ? '#9aa6b4' : PALETTE.rampHi;
  ctx.fillRect((x - 16) | 0, (y + 6) | 0, 32, 2);
}

function drawRider(ctx, s) {
  const r = s.rider;
  if (!r.alive && r.respawn > 0) return;
  if (r.hitFlash > 0 && Math.floor(r.hitFlash * 14) % 2 === 0) return;
  const x = r.x, y = RIDER_Y - (r.air > 0 ? 14 : 0);
  // Shadow on the snow when airborne.
  if (r.air > 0) {
    ctx.fillStyle = 'rgba(0,0,0,0.18)';
    ctx.fillRect((x - 10) | 0, RIDER_Y + 8, 20, 4);
  }
  // Board.
  ctx.fillStyle = PALETTE.board;
  ctx.fillRect((x - 11) | 0, (y + 9) | 0, 22, 5);
  // Body.
  ctx.fillStyle = PALETTE.riderLo;
  ctx.fillRect((x - 8) | 0, (y - 10) | 0, 16, 20);
  ctx.fillStyle = PALETTE.rider;
  ctx.fillRect((x - 7) | 0, (y - 9) | 0, 14, 18);
  ctx.fillStyle = PALETTE.riderHi;
  ctx.fillRect((x - 7) | 0, (y - 9) | 0, 14, 3);
  // Head.
  ctx.fillStyle = '#e8b890';
  ctx.fillRect((x - 4) | 0, (y - 17) | 0, 8, 8);
  // Goggles.
  ctx.fillStyle = '#3a4a6a';
  ctx.fillRect((x - 4) | 0, (y - 15) | 0, 8, 3);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) drawHeart(ctx, 124 + i * 12, 16);
  ctx.textAlign = 'center';
  // Slope progress bar.
  const prog = Math.min(1, s.worldY / s.cfg.length);
  ctx.fillStyle = '#0e1422';
  ctx.fillRect(VW / 2 - 30, 12, 60, 7);
  ctx.fillStyle = '#5fc0ff';
  ctx.fillRect(VW / 2 - 29, 13, 58 * prog, 5);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText(t(lang, 'score') + ' ' + Math.round(s.score), VW - 6, 16);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.55);
  ctx.fillStyle = s.won ? `rgba(95,192,110,${0.35 * a})` :
                  (s.rider && !s.rider.alive) ? `rgba(255,80,80,${0.45 * a})` :
                                                `rgba(255,255,255,${0.3 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
