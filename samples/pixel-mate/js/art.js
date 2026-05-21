// Pixel-art rendering for Pixel Mate. 360x480 world units.

const PALETTE = {
  bg:        '#1a1224',
  bgHi:      '#28203a',
  light:     '#f0d6a8',
  dark:      '#7a4a1f',
  edge:      '#070315',
  select:    'rgba(255,224,80,0.55)',
  selectRing:'#ffe04a',
  target:    'rgba(95,192,110,0.45)',
  targetRing:'#5fc06e',
  white:     '#fdf6e3',
  whiteEdge: '#7a6048',
  black:     '#1a1224',
  blackEdge: '#070315',
  glyph:     '#ffe04a',
  hud:       '#070315',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  star:      '#f8d34a',
  starOff:   '#3a4274',
  win:       '#5fc06e',
};

// Board geometry — cell 40 px ⇒ 8×40 = 320 px wide, centred.
const CELL = 40;
const BOARD_OX = 20;
const BOARD_OY = 60;

function cellRect(x, y) {
  return { x: BOARD_OX + x * CELL, y: BOARD_OY + y * CELL, w: CELL, h: CELL };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgHi;
  for (let i = 0; i < 26; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s) {
  ctx.fillStyle = PALETTE.edge;
  ctx.fillRect(BOARD_OX - 4, BOARD_OY - 4, CELL * 8 + 8, CELL * 8 + 8);
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
    const r = cellRect(x, y);
    ctx.fillStyle = ((x + y) % 2 === 0) ? PALETTE.light : PALETTE.dark;
    ctx.fillRect(r.x, r.y, r.w, r.h);
  }
  // Move targets BELOW pieces so the piece sits on top.
  if (s.selected) {
    for (const m of s.moves) {
      const r = cellRect(m.x, m.y);
      ctx.fillStyle = PALETTE.target;
      ctx.fillRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
      ctx.strokeStyle = PALETTE.targetRing;
      ctx.lineWidth = 2;
      ctx.strokeRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
    }
    const sel = cellRect(s.selected.x, s.selected.y);
    ctx.fillStyle = PALETTE.select;
    ctx.fillRect(sel.x + 2, sel.y + 2, sel.w - 4, sel.h - 4);
    ctx.strokeStyle = PALETTE.selectRing;
    ctx.lineWidth = 2;
    ctx.strokeRect(sel.x + 2, sel.y + 2, sel.w - 4, sel.h - 4);
  }
  // Pieces.
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
    const sq = s.board[y][x];
    if (sq) drawPiece(ctx, x, y, sq);
  }
}

function drawPiece(ctx, x, y, piece) {
  const r = cellRect(x, y);
  const cx = r.x + r.w / 2, cy = r.y + r.h / 2 + 1;
  const glyph = { K: '♔', Q: '♕', R: '♖', B: '♗', N: '♘', P: '♙' }[piece.p];
  if (piece.c === 'w') {
    // White: light fill with dark outline + glyph in dark.
    ctx.fillStyle = PALETTE.whiteEdge;
    ctx.font = 'bold 30px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const [dx, dy] of [[-1,0],[1,0],[0,-1],[0,1]]) ctx.fillText(glyph, cx + dx, cy + dy);
    ctx.fillStyle = PALETTE.white;
    ctx.fillText(glyph, cx, cy);
  } else {
    ctx.fillStyle = PALETTE.black;
    ctx.font = 'bold 30px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(glyph, cx, cy);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'attempts') + ' ' + s.attempts, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText('best ' + (best == null ? '—' : best), VW - 6, 16);
}

function drawStars(ctx, x, y, n, w = 14) {
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < n ? PALETTE.star : PALETTE.starOff;
    drawStar(ctx, x + i * (w + 4) + w / 2, y, w / 2);
  }
}
function drawStar(ctx, cx, cy, r) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const a = (Math.PI / 5) * i - Math.PI / 2;
    const rr = i % 2 === 0 ? r : r * 0.45;
    ctx.lineTo(cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
  }
  ctx.closePath();
  ctx.fill();
}
