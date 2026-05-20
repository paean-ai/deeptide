// Pixel-art rendering for Pixel Pac-Pixel. 360x480 world units.

const PALETTE = {
  bg:        '#0a0a18',
  wall:      '#2845c8',
  wallHi:    '#5a72e8',
  wallLo:    '#0e1860',
  corridor:  '#0a0a18',
  pellet:    '#f0d6a8',
  power:     '#ffd34a',
  pac:       '#ffe04a',
  pacHi:     '#fff0c0',
  pacShadow: '#a07a14',
  blinky:    '#ff5a5a',
  pinky:     '#ffaad8',
  ghostHi:   '#ffd0e0',
  panic:     '#3a52d8',
  panicHi:   '#7a92ff',
  eyeWhite:  '#fff7ed',
  eyePupil:  '#0a0a18',
  hud:       '#06061a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  heart:     '#ff5a6e',
  warn:      '#ff5a3a',
  win:       '#5fc06e',
};

function cellRect(c, r) {
  return { x: BOARD_OX + c * CELL, y: BOARD_OY + r * CELL, w: CELL, h: CELL };
}
function cellRectF(cf, rf) {
  return { x: BOARD_OX + cf * CELL, y: BOARD_OY + rf * CELL, w: CELL, h: CELL };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawMaze(ctx, s) {
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    const ch = s.maze[r][c];
    const rect = cellRect(c, r);
    if (ch === ' ') {
      ctx.fillStyle = PALETTE.wall;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      ctx.fillStyle = PALETTE.wallLo;
      ctx.fillRect(rect.x, rect.y + rect.h - 2, rect.w, 2);
      ctx.fillStyle = PALETTE.wallHi;
      ctx.fillRect(rect.x, rect.y, rect.w, 2);
    } else if (ch === '.') {
      ctx.fillStyle = PALETTE.pellet;
      ctx.fillRect(rect.x + rect.w / 2 - 1, rect.y + rect.h / 2 - 1, 3, 3);
    } else if (ch === 'o') {
      // Power-pellet pulses.
      const pulse = 4 + Math.floor((s.elapsed * 6) % 2) * 2;
      ctx.fillStyle = PALETTE.power;
      drawDisk(ctx, rect.x + rect.w / 2, rect.y + rect.h / 2, pulse);
    } else if (ch === 'h') {
      // Faint home tile.
      ctx.fillStyle = '#1c1838';
      ctx.fillRect(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4);
    } else if (ch === 'T') {
      // Tunnel exit indicator.
      ctx.fillStyle = '#1c1838';
      ctx.fillRect(rect.x, rect.y + 2, rect.w, rect.h - 4);
    }
  }
}

function drawDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}

function drawPac(ctx, p, t) {
  const cx = BOARD_OX + p.x * CELL;
  const cy = BOARD_OY + p.y * CELL;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return;
  ctx.fillStyle = PALETTE.pacShadow;
  drawDisk(ctx, cx, cy + 1, CELL * 0.36);
  ctx.fillStyle = PALETTE.pac;
  drawDisk(ctx, cx, cy, CELL * 0.34);
  ctx.fillStyle = PALETTE.pacHi;
  drawDisk(ctx, cx - 2, cy - 2, CELL * 0.20);
  // Mouth — opens / closes with mouthT, facing dir
  const open = (Math.sin(p.mouthT * 18) + 1) * 0.5;
  if (open > 0.1 && (p.dir.x || p.dir.y)) {
    const ang = Math.atan2(p.dir.y, p.dir.x);
    ctx.fillStyle = PALETTE.bg;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, CELL * 0.34 + 1, ang - open * 0.6, ang + open * 0.6);
    ctx.closePath();
    ctx.fill();
  }
}

function drawGhost(ctx, g, panicT, panicMax) {
  const cx = BOARD_OX + g.x * CELL;
  const cy = BOARD_OY + g.y * CELL;
  const r  = CELL * 0.36;
  let body = g.color, hi = PALETTE.ghostHi;
  if (g.mode === 'panic') {
    // Flash white near the end of panic.
    const blink = panicT < 2 && Math.floor(panicT * 6) % 2 === 0;
    body = blink ? '#f4f4ff' : PALETTE.panic;
    hi = blink ? '#ffffff' : PALETTE.panicHi;
  } else if (g.mode === 'eyes') {
    body = null; hi = null;
  }
  if (body) {
    // Dome
    ctx.fillStyle = body;
    drawDisk(ctx, cx, cy - 2, r);
    ctx.fillRect((cx - r) | 0, cy - 2, (r * 2 + 1) | 0, r + 1);
    // Skirt — 3 zig-zags
    const baseY = cy + r - 1;
    for (let i = -3; i <= 2; i++) {
      const sx = cx + i * 4;
      const dip = (i % 2 === 0) ? 2 : 4;
      ctx.fillRect(sx, baseY, 4, dip);
    }
    // Highlight
    if (hi) {
      ctx.fillStyle = hi;
      drawDisk(ctx, cx - 3, cy - 4, r * 0.45);
    }
  }
  // Eyes — always shown (panic = pure white circles with no pupils)
  ctx.fillStyle = PALETTE.eyeWhite;
  drawDisk(ctx, cx - 3, cy - 1, 3);
  drawDisk(ctx, cx + 3, cy - 1, 3);
  if (g.mode !== 'panic') {
    ctx.fillStyle = PALETTE.eyePupil;
    const dx = Math.sign(g.dir.x), dy = Math.sign(g.dir.y);
    ctx.fillRect(cx - 3 + dx, cy - 1 + dy, 2, 2);
    ctx.fillRect(cx + 3 + dx, cy - 1 + dy, 2, 2);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, BOARD_OY);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) {
    drawHeart(ctx, 120 + i * 12, 16);
  }
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'pellets') + ' ' + (s.pellets + s.power - s.pelletsLeft - s.powerLeft) + '/' + (s.pellets + s.power), VW / 2 + 16, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
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
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.4 * a})` :
                  !s.pac.alive ? `rgba(255,80,80,${0.5 * a})` :
                                 `rgba(255,255,255,${0.25 * a})`;
  ctx.fillRect(0, BOARD_OY, VW, ROWS * CELL);
}
