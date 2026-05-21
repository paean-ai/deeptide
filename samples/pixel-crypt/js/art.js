// Pixel-art rendering for Pixel Crypt. 360x480 world units.

const PALETTE = {
  bg:       '#16131f',
  hidden:   '#4a4258',
  hiddenHi: '#645a78',
  hiddenLo: '#2e2940',
  open:     '#241f30',
  openEdge: '#352e44',
  flag:     '#e8554f',
  monLv:    ['#000000', '#5fc06e', '#4aa6e0', '#f4c44a', '#f0883a', '#e8554f'],
  clueLo:   '#8fd0a0',
  clueMid:  '#f4c44a',
  clueHi:   '#f0883a',
  clueMax:  '#e8554f',
  hud:      '#0d0b14',
  hudText:  '#f3f1e6',
  hp:       '#e8554f',
  hpBg:     '#3a2030',
  xp:       '#5f9bd0',
  xpBg:     '#1f2c3c',
  accent:   '#f4c44a',
  good:     '#5fc06e',
  hit:      'rgba(232,85,79,0.55)',
};

function gridGeometry(n) {
  const cell = Math.min(36, (300 / n) | 0);
  const span = cell * n;
  const ox = ((VW - span) / 2) | 0;
  const oy = 96;
  return { cell, span, ox, oy };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function clueColor(v) {
  if (v <= 2) return PALETTE.clueLo;
  if (v <= 5) return PALETTE.clueMid;
  if (v <= 9) return PALETTE.clueHi;
  return PALETTE.clueMax;
}

function drawMonster(ctx, x, y, cell, level) {
  const col = PALETTE.monLv[level] || '#e8554f';
  const cx = x + cell / 2, cy = y + cell / 2 + 1;
  const r = cell * 0.3;
  ctx.fillStyle = col;
  ctx.fillRect((cx - r) | 0, (cy - r) | 0, (r * 2) | 0, (r * 2) | 0);
  ctx.fillStyle = '#0d0b14';
  ctx.fillRect((cx - r + 2) | 0, (cy - 2) | 0, 2, 3);
  ctx.fillRect((cx + r - 4) | 0, (cy - 2) | 0, 2, 3);
  ctx.fillStyle = '#fff';
  ctx.font = 'bold 9px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(String(level), cx, y + cell - 2);
}

function drawBoard(ctx, s) {
  const g = gridGeometry(s.n);
  ctx.fillStyle = PALETTE.hiddenLo;
  ctx.fillRect(g.ox - 4, g.oy - 4, g.span + 8, g.span + 8);
  for (let i = 0; i < s.n * s.n; i++) {
    const r = (i / s.n) | 0, c = i % s.n;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    if (!s.revealed[i]) {
      ctx.fillStyle = PALETTE.hiddenLo;
      ctx.fillRect(x, y, g.cell, g.cell);
      ctx.fillStyle = PALETTE.hidden;
      ctx.fillRect(x + 1, y + 1, g.cell - 3, g.cell - 3);
      ctx.fillStyle = PALETTE.hiddenHi;
      ctx.fillRect(x + 1, y + 1, g.cell - 3, 2);
      if (s.flagged[i]) {
        ctx.fillStyle = '#3a3a3a';
        ctx.fillRect(x + g.cell / 2 - 1, y + 6, 2, g.cell - 12);
        ctx.fillStyle = PALETTE.flag;
        ctx.fillRect(x + g.cell / 2 + 1, y + 6, g.cell / 2 - 5, 6);
      }
    } else {
      ctx.fillStyle = PALETTE.open;
      ctx.fillRect(x, y, g.cell, g.cell);
      ctx.fillStyle = PALETTE.openEdge;
      ctx.fillRect(x, y, g.cell, 1);
      if (s.monster[i] > 0) {
        drawMonster(ctx, x, y, g.cell, s.monster[i]);
      } else if (s.clue[i] > 0) {
        ctx.fillStyle = clueColor(s.clue[i]);
        ctx.font = 'bold ' + Math.min(16, (g.cell * 0.5) | 0) + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(String(s.clue[i]), x + g.cell / 2, y + g.cell / 2 + 1);
      }
    }
  }
  // Flash the tile of the most recent wounding fight.
  if (s.lastHit >= 0 && s.revealed[s.lastHit]) {
    const r = (s.lastHit / s.n) | 0, c = s.lastHit % s.n;
    ctx.fillStyle = PALETTE.hit;
    ctx.fillRect(g.ox + c * g.cell, g.oy + r * g.cell, g.cell, g.cell);
  }
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 88);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 14);
  ctx.textAlign = 'right';
  ctx.fillText('☠ ' + s.slain + '/' + s.monsterTotal, VW - 8, 14);
  // HP bar.
  ctx.textAlign = 'left';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText('HP', 8, 36);
  ctx.fillStyle = PALETTE.hpBg;
  ctx.fillRect(34, 30, 200, 12);
  ctx.fillStyle = PALETTE.hp;
  ctx.fillRect(34, 30, 200 * Math.max(0, s.hp) / MAX_HP, 12);
  ctx.fillStyle = PALETTE.hudText;
  ctx.textAlign = 'right';
  ctx.fillText(s.hp + '/' + MAX_HP, VW - 8, 36);
  // Hero level + XP progress to next.
  ctx.textAlign = 'left';
  ctx.fillStyle = PALETTE.accent;
  ctx.fillText(t(lang, 'lvl') + ' ' + s.level, 8, 60);
  const cur = XP_FOR_LEVEL[s.level] || 0;
  const nxt = XP_FOR_LEVEL[s.level + 1];
  ctx.fillStyle = PALETTE.xpBg;
  ctx.fillRect(64, 54, 170, 12);
  if (nxt !== undefined) {
    ctx.fillStyle = PALETTE.xp;
    const frac = Math.max(0, Math.min(1, (s.xp - cur) / (nxt - cur)));
    ctx.fillRect(64, 54, 170 * frac, 12);
  } else {
    ctx.fillStyle = PALETTE.accent;
    ctx.fillRect(64, 54, 170, 12);
  }
  ctx.fillStyle = PALETTE.hudText;
  ctx.textAlign = 'right';
  ctx.fillText('XP ' + s.xp, VW - 8, 60);
}

function drawTitleArt(ctx, cx, cy) {
  const cell = 34, n = 3, ox = cx - cell * n / 2, oy = cy - cell * n / 2;
  const demo = [3, 0, 1, 0, 0, 2, 1, 0, 0];           // 0 = open clue tile
  for (let i = 0; i < 9; i++) {
    const x = ox + (i % n) * cell, y = oy + ((i / n) | 0) * cell;
    if (demo[i] > 0) {
      ctx.fillStyle = PALETTE.open;
      ctx.fillRect(x, y, cell, cell);
      drawMonster(ctx, x, y, cell, demo[i]);
    } else {
      ctx.fillStyle = PALETTE.hiddenLo; ctx.fillRect(x, y, cell, cell);
      ctx.fillStyle = PALETTE.hidden;   ctx.fillRect(x + 1, y + 1, cell - 3, cell - 3);
      ctx.fillStyle = PALETTE.hiddenHi; ctx.fillRect(x + 1, y + 1, cell - 3, 2);
    }
  }
}
