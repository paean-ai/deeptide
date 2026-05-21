// Pixel-art rendering for Pixel Leap. 360x480 world units.

const PALETTE = {
  bg:       '#171a2b',
  bgGlow:   '#202544',
  solid:    '#4a5278',
  solidHi:  '#6b74a0',
  solidLo:  '#2e3450',
  spike:    '#d8505a',
  spikeHi:  '#ff7a82',
  exit:     '#5fd07a',
  exitHi:   '#9bf0ad',
  gem:      '#f4c44a',
  gemHi:    '#fff0b0',
  hero:     '#46b8e8',
  heroHi:   '#9be0ff',
  heroDk:   '#1f6a96',
  skin:     '#e8b890',
  dashGlow: 'rgba(155,224,255,0.5)',
  hud:      '#0e0f1a',
  hudText:  '#f3f1e6',
  accent:   '#f4c44a',
  good:     '#5fd07a',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgGlow;
  for (let i = 0; i < 48; i++) ctx.fillRect((i * 71 + 23) % VW, (i * 53 + 17) % VH, 2, 2);
}

function drawLevel(ctx, s) {
  // Tiles.
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const ch = s.grid[r][c];
      const x = OX + c * TILE, y = OY + r * TILE;
      if (ch === '#') {
        ctx.fillStyle = PALETTE.solidLo;
        ctx.fillRect(x, y, TILE, TILE);
        ctx.fillStyle = PALETTE.solid;
        ctx.fillRect(x, y, TILE - 1, TILE - 1);
        ctx.fillStyle = PALETTE.solidHi;
        ctx.fillRect(x, y, TILE - 1, 2);
      } else if (ch === '^') {
        ctx.fillStyle = PALETTE.spike;
        for (let k = 0; k < 4; k++) {
          const sx = x + k * 6;
          ctx.beginPath();
          ctx.moveTo(sx, y + TILE);
          ctx.lineTo(sx + 3, y + TILE - 13);
          ctx.lineTo(sx + 6, y + TILE);
          ctx.closePath();
          ctx.fill();
        }
        ctx.fillStyle = PALETTE.spikeHi;
        for (let k = 0; k < 4; k++) ctx.fillRect(x + k * 6 + 2, y + TILE - 11, 1, 8);
      }
    }
  }
  // Exit door.
  {
    const x = OX + s.exit.c * TILE, y = OY + s.exit.r * TILE;
    ctx.fillStyle = PALETTE.exit;
    ctx.fillRect(x + 3, y + 2, TILE - 6, TILE - 2);
    ctx.fillStyle = PALETTE.exitHi;
    ctx.fillRect(x + 5, y + 4, TILE - 10, 4);
    ctx.fillRect(x + TILE / 2 - 1, y + 4, 2, TILE - 8);
  }
  // Gems.
  for (const g of s.gems) {
    if (g.got) continue;
    const cx = OX + g.c * TILE + TILE / 2, cy = OY + g.r * TILE + TILE / 2;
    ctx.fillStyle = PALETTE.gem;
    ctx.beginPath();
    ctx.moveTo(cx, cy - 7); ctx.lineTo(cx + 6, cy); ctx.lineTo(cx, cy + 7); ctx.lineTo(cx - 6, cy);
    ctx.closePath(); ctx.fill();
    ctx.fillStyle = PALETTE.gemHi;
    ctx.fillRect(cx - 2, cy - 3, 2, 2);
  }
  // Hero.
  if (s.hero) drawHero(ctx, s);
}

function drawHero(ctx, s) {
  const h = s.hero;
  if (h.dashT > 0) {
    ctx.fillStyle = PALETTE.dashGlow;
    ctx.fillRect(h.x - s.facing * 10, h.y + 2, HERO_W + 10, HERO_H - 4);
  }
  // Body.
  ctx.fillStyle = PALETTE.heroDk;
  ctx.fillRect(h.x, h.y, HERO_W, HERO_H);
  ctx.fillStyle = PALETTE.hero;
  ctx.fillRect(h.x + 1, h.y + 6, HERO_W - 2, HERO_H - 7);
  ctx.fillStyle = PALETTE.heroHi;
  ctx.fillRect(h.x + 1, h.y + 6, 3, HERO_H - 7);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect(h.x + 2, h.y, HERO_W - 4, 8);
  // Eye toward facing.
  ctx.fillStyle = '#1d1518';
  ctx.fillRect(h.x + (s.facing > 0 ? HERO_W - 6 : 3), h.y + 3, 2, 2);
  // Feet.
  ctx.fillStyle = PALETTE.heroDk;
  ctx.fillRect(h.x + 1, h.y + HERO_H - 3, 4, 3);
  ctx.fillRect(h.x + HERO_W - 5, h.y + HERO_H - 3, 4, 3);
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, OY);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, OY / 2);
  ctx.textAlign = 'center';
  const got = s.gems.filter(g => g.got).length;
  if (s.gems.length) {
    ctx.fillStyle = PALETTE.gem;
    ctx.fillText('◆ ' + got + '/' + s.gems.length, VW / 2, OY / 2);
  }
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText(t(lang, 'deaths') + ' ' + s.deaths, VW - 8, OY / 2);
}

function drawTitleArt(ctx, cx, cy) {
  // A few solid blocks, a spike, and the hero mid-leap.
  for (let i = 0; i < 5; i++) {
    const x = cx - 60 + i * 24, y = cy + 30;
    ctx.fillStyle = PALETTE.solidLo; ctx.fillRect(x, y, 24, 24);
    ctx.fillStyle = PALETTE.solid;   ctx.fillRect(x, y, 23, 23);
    ctx.fillStyle = PALETTE.solidHi; ctx.fillRect(x, y, 23, 2);
  }
  ctx.fillStyle = PALETTE.spike;
  for (let k = 0; k < 4; k++) {
    const sx = cx - 12 + k * 6;
    ctx.beginPath();
    ctx.moveTo(sx, cy + 30); ctx.lineTo(sx + 3, cy + 18); ctx.lineTo(sx + 6, cy + 30);
    ctx.closePath(); ctx.fill();
  }
  drawHero(ctx, { hero: { x: cx - 7, y: cy - 14, dashT: 0 }, facing: 1 });
}
