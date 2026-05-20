// Pixel-art rendering for Pixel Centipede. 360x480 world units.

const PALETTE = {
  bg:        '#0a0d1e',
  bgGlow:    '#101630',
  hud:       '#06081a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  divider:   '#202842',
  playerZone:'#101a2e',
  mushBody:  '#7ac96a',
  mushBodyD: '#4d9a44',
  mushCap:   '#e85a3a',
  mushCap2:  '#c83a2a',
  mushHit:   '#f7c93a',
  mushDot:   '#fff7ed',
  cent:      '#7fd84a',
  centHi:    '#bce088',
  centHead:  '#ffe07a',
  centHeadHi:'#fff4c0',
  bullet:    '#ffd34a',
  bulletHi:  '#fff0c8',
  spider:    '#bda6ff',
  spiderHi:  '#e3d3ff',
  spiderEye: '#0a0a0a',
  player:    '#5fc0ff',
  playerHi:  '#a8e0ff',
  playerLo:  '#205a8a',
  border:    '#070a18',
  warn:      '#ff5a5a',
  heart:     '#ff4a5a',
};

const TILE = CELL;

function cellRect(col, row) {
  return {
    x: BOARD_OX + col * TILE, y: BOARD_OY + row * TILE,
    w: TILE, h: TILE,
  };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // Player zone tint at the bottom rows.
  ctx.fillStyle = PALETTE.playerZone;
  ctx.fillRect(0, BOARD_OY + (ROWS - PLAYER_ROWS) * TILE, VW, PLAYER_ROWS * TILE);
  ctx.fillStyle = PALETTE.divider;
  ctx.fillRect(0, BOARD_OY + (ROWS - PLAYER_ROWS) * TILE, VW, 1);
  // Border around the playfield.
  ctx.fillStyle = PALETTE.border;
  ctx.fillRect(0, BOARD_OY, VW, 1);
  ctx.fillRect(0, BOARD_OY + ROWS * TILE - 1, VW, 1);
}

function drawMushrooms(ctx, mushrooms) {
  for (let y = 0; y < ROWS; y++) for (let x = 0; x < COLS; x++) {
    const hp = mushrooms[y][x];
    if (!hp) continue;
    const r = cellRect(x, y);
    const cx = r.x + TILE / 2, cy = r.y + TILE / 2;
    // Stem
    ctx.fillStyle = PALETTE.mushBody;
    ctx.fillRect(cx - 4, cy + 1, 8, 6);
    ctx.fillStyle = PALETTE.mushBodyD;
    ctx.fillRect(cx - 4, cy + 5, 8, 2);
    // Cap - colour drains with damage
    const capCol = hp === 3 ? PALETTE.mushCap :
                   hp === 2 ? PALETTE.mushCap2 : PALETTE.mushHit;
    ctx.fillStyle = capCol;
    ctx.fillRect(cx - 7, cy - 5, 14, 6);
    ctx.fillRect(cx - 5, cy - 7, 10, 2);
    // dots
    ctx.fillStyle = PALETTE.mushDot;
    ctx.fillRect(cx - 4, cy - 3, 2, 2);
    ctx.fillRect(cx + 2, cy - 4, 2, 2);
  }
}

function drawSegment(ctx, seg, headOfWorm) {
  const r = cellRect(seg.col, seg.row);
  const cx = r.x + TILE / 2, cy = r.y + TILE / 2;
  const body = headOfWorm ? PALETTE.centHead : PALETTE.cent;
  const hi   = headOfWorm ? PALETTE.centHeadHi : PALETTE.centHi;
  // body
  ctx.fillStyle = body;
  fillDisk(ctx, cx, cy, 8);
  ctx.fillStyle = hi;
  fillDisk(ctx, cx - 2, cy - 2, 4);
  ctx.fillStyle = body;
  fillDisk(ctx, cx + 1, cy + 1, 3);
  // legs
  ctx.fillStyle = hi;
  ctx.fillRect(cx - 8, cy + 6, 2, 2);
  ctx.fillRect(cx + 6, cy + 6, 2, 2);
  if (headOfWorm) {
    // mandibles
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(cx - 7, cy + 4, 2, 2);
    ctx.fillRect(cx + 5, cy + 4, 2, 2);
    // eyes
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(cx - 4, cy - 4, 2, 2);
    ctx.fillRect(cx + 2, cy - 4, 2, 2);
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

function drawBullet(ctx, b) {
  const r = cellRect(b.col, b.row);
  ctx.fillStyle = PALETTE.bullet;
  ctx.fillRect(r.x + TILE / 2 - 1, r.y + TILE / 2 - 4, 2, 8);
  ctx.fillStyle = PALETTE.bulletHi;
  ctx.fillRect(r.x + TILE / 2 - 1, r.y + TILE / 2 - 4, 2, 2);
}

function drawPlayer(ctx, p) {
  const r = cellRect(p.col, p.row);
  const cx = r.x + TILE / 2, cy = r.y + TILE / 2;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return; // blink
  ctx.fillStyle = PALETTE.playerLo;
  ctx.fillRect(cx - 7, cy - 4, 14, 9);
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(cx - 6, cy - 3, 12, 6);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(cx - 6, cy - 3, 12, 2);
  // Cannon barrel
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(cx - 2, cy - 8, 4, 5);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(cx - 1, cy - 8, 2, 2);
}

function drawSpider(ctx, sp) {
  const r = cellRect(sp.col, sp.row);
  const cx = r.x + TILE / 2, cy = r.y + TILE / 2;
  const wob = Math.sin(sp.anim * 16) * 1;
  ctx.fillStyle = '#1a1730';
  fillDisk(ctx, cx, cy + 1, 8);
  ctx.fillStyle = PALETTE.spider;
  fillDisk(ctx, cx, cy, 7);
  ctx.fillStyle = PALETTE.spiderHi;
  fillDisk(ctx, cx - 1, cy - 1, 3);
  // 8 legs
  ctx.fillStyle = PALETTE.spider;
  for (let i = 0; i < 4; i++) {
    const lo = 6 + wob + i;
    ctx.fillRect(cx - 10, cy - 3 + i * 2, 3, 1);
    ctx.fillRect(cx + 7,  cy - 3 + i * 2, 3, 1);
  }
  // eyes
  ctx.fillStyle = PALETTE.spiderEye;
  ctx.fillRect(cx - 3, cy - 2, 2, 2);
  ctx.fillRect(cx + 1, cy - 2, 2, 2);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, BOARD_OY);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'wave') + ' ' + (s.waveIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
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

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.4);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.player.hitFlash > 0 ? `rgba(255,80,80,${0.5 * a})` :
                                          `rgba(255,255,255,${0.15 * a})`;
  ctx.fillRect(0, BOARD_OY, VW, ROWS * TILE);
}
