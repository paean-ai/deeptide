// Pixel-art rendering for Pixel Binairo. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  card:      '#1c2240',
  cardEdge:  '#070b1a',
  cellEmpty: '#2a3258',
  cellEmptyHi:'#384268',
  zero:      '#3a4274',          // a '0' tile (cool slate)
  zeroHi:    '#5a64a0',
  one:       '#ff9b3e',          // a '1' tile (warm amber)
  oneHi:     '#ffc070',
  fixedRing: '#f8f5e8',
  border:    '#070b1a',
  conflict:  'rgba(232,85,79,0.5)',
  hud:       '#070b1a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  ok:        '#5fc06e',
};

function gridGeometry(n) {
  const cell = n <= 6 ? 44 : n <= 8 ? 34 : 28;
  const total = cell * n;
  const ox = ((VW - total) / 2) | 0;
  const oy = 64;
  return { cell, total, ox, oy };
}
function cellRect(n, c, r) {
  const g = gridGeometry(n);
  return { x: g.ox + c * g.cell, y: g.oy + r * g.cell, w: g.cell, h: g.cell };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = '#161d3a';
  for (let i = 0; i < 24; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawScene(ctx, s, bad) {
  const n = s.n;
  const g = gridGeometry(n);
  ctx.fillStyle = PALETTE.cardEdge;
  ctx.fillRect(g.ox - 4, g.oy - 4, g.total + 8, g.total + 8);
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(g.ox - 2, g.oy - 2, g.total + 4, g.total + 4);
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) {
    const i = r * n + c;
    const v = s.grid[i];
    const rect = cellRect(n, c, r);
    ctx.fillStyle = PALETTE.border;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    if (v === EMPTY) {
      ctx.fillStyle = ((r + c) % 2) ? PALETTE.cellEmpty : PALETTE.cellEmptyHi;
      ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
    } else {
      const body = v === 0 ? PALETTE.zero : PALETTE.one;
      const hi   = v === 0 ? PALETTE.zeroHi : PALETTE.oneHi;
      ctx.fillStyle = body;
      ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
      ctx.fillStyle = hi;
      ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, 3);
      // Digit glyph.
      ctx.fillStyle = '#0d1228';
      ctx.font = 'bold ' + (g.cell * 0.5 | 0) + 'px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(v), rect.x + rect.w / 2, rect.y + rect.h / 2 + 1);
    }
    // Fixed-clue ring.
    if (s.fixed[i]) {
      ctx.strokeStyle = PALETTE.fixedRing;
      ctx.lineWidth = 1.5;
      ctx.strokeRect(rect.x + 2.5, rect.y + 2.5, rect.w - 5, rect.h - 5);
    }
    // Conflict tint.
    if (bad && bad[i]) {
      ctx.fillStyle = PALETTE.conflict;
      ctx.fillRect(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2);
    }
  }
}

function drawHud(ctx, lang, s, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  let filled = 0;
  for (const v of s.grid) if (v !== EMPTY) filled++;
  ctx.fillText(filled + '/' + (s.n * s.n), VW / 2, 16);
  ctx.textAlign = 'right';
  const mm = (elapsedSec / 60) | 0, ss = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + mm + ':' + String(ss).padStart(2, '0'), VW - 6, 16);
}
