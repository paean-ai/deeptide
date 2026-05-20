// Pixel-art rendering for Pixel Bloxorz. 360x480 world units.

const PALETTE = {
  bg:        '#10141e',
  card:      '#1a2030',
  cardEdge:  '#070b18',
  tile:      '#4a3622',
  tileHi:    '#6d502f',
  tileLo:    '#2f2114',
  tileBorder:'#1c130a',
  weak:      '#834c30',
  weakHi:    '#a7654a',
  weakDot:   '#5e2f1c',
  goal:      '#0c0c1a',
  goalRim:   '#5fc06e',
  block:     '#f4d27b',
  blockHi:   '#fff0c8',
  blockLo:   '#a07a14',
  blockEdge: '#5a4014',
  shadow:    'rgba(0,0,0,0.42)',
  hud:       '#070b18',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  warn:      '#ff5a3a',
  win:       '#5fc06e',
  star:      '#f8d34a',
  starOff:   '#3a4274',
};

// Geometry: tile size adapts to the board so anything up to 9x8 fits in
// the 360x440 board area. Board sits at oy = 72 to leave room for HUD +
// control hints.
function gridGeometry(w, h) {
  const cell = Math.min(56, Math.floor(330 / w), Math.floor(340 / h));
  const ox = ((360 - cell * w) / 2) | 0;
  const oy = 72;
  return { cell, ox, oy };
}
function cellRect(g, c, r) {
  return { x: g.ox + c * g.cell, y: g.oy + r * g.cell, w: g.cell, h: g.cell };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawBoard(ctx, s) {
  const g = gridGeometry(s.w, s.h);
  for (let r = 0; r < s.h; r++) for (let c = 0; c < s.w; c++) {
    const t = s.grid[r][c];
    if (t === ' ') continue;
    drawTile(ctx, g, c, r, t, c === s.goal.col && r === s.goal.row);
  }
}

function drawTile(ctx, g, c, r, t, isGoal) {
  const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
  ctx.fillStyle = PALETTE.tileBorder;
  ctx.fillRect(x, y, g.cell, g.cell);
  ctx.fillStyle = (t === 'w') ? PALETTE.weak : PALETTE.tile;
  ctx.fillRect(x + 1, y + 1, g.cell - 2, g.cell - 2);
  // Top bevel
  ctx.fillStyle = (t === 'w') ? PALETTE.weakHi : PALETTE.tileHi;
  ctx.fillRect(x + 1, y + 1, g.cell - 2, 3);
  ctx.fillRect(x + 1, y + 1, 3, g.cell - 2);
  // Bottom shadow
  ctx.fillStyle = PALETTE.tileLo;
  ctx.fillRect(x + 1, y + g.cell - 4, g.cell - 2, 3);
  ctx.fillRect(x + g.cell - 4, y + 1, 3, g.cell - 2);
  if (t === 'w') {
    // Cracked dots to suggest fragility.
    ctx.fillStyle = PALETTE.weakDot;
    ctx.fillRect(x + 6, y + 6, 2, 2);
    ctx.fillRect(x + g.cell - 8, y + 8, 2, 2);
    ctx.fillRect(x + g.cell / 2, y + g.cell / 2, 2, 2);
  }
  if (isGoal) {
    // Carve a dark goal hole in the centre and outline it with a green rim.
    const m = (g.cell * 0.32) | 0;
    ctx.fillStyle = PALETTE.goal;
    ctx.fillRect(x + m, y + m, g.cell - 2 * m, g.cell - 2 * m);
    ctx.strokeStyle = PALETTE.goalRim;
    ctx.lineWidth = 2;
    ctx.strokeRect(x + m, y + m, g.cell - 2 * m, g.cell - 2 * m);
  }
}

function drawBlock(ctx, s) {
  const g = gridGeometry(s.w, s.h);
  const b = s.block;
  // Drop a shadow under the footprint.
  ctx.fillStyle = PALETTE.shadow;
  if (b.orient === 'up') {
    const rr = cellRect(g, b.col, b.row);
    ctx.fillRect(rr.x + 4, rr.y + g.cell - 5, g.cell - 6, 4);
  } else if (b.orient === 'h') {
    const rr = cellRect(g, b.col, b.row);
    ctx.fillRect(rr.x + 4, rr.y + g.cell - 5, g.cell * 2 - 6, 4);
  } else {
    const rr = cellRect(g, b.col, b.row);
    ctx.fillRect(rr.x + 4, rr.y + g.cell - 5, g.cell - 6, 4);
    ctx.fillRect(rr.x + 4, rr.y + g.cell * 2 - 5, g.cell - 6, 4);
  }
  // Block body — render as an isometric-ish tall prism on standing, and a
  // squat slab when lying.
  if (b.orient === 'up') {
    const rr = cellRect(g, b.col, b.row);
    drawStanding(ctx, rr.x + 2, rr.y - g.cell + 4, g.cell - 4, g.cell * 2 - 8);
  } else if (b.orient === 'h') {
    const rr = cellRect(g, b.col, b.row);
    drawSlab(ctx, rr.x + 2, rr.y + 4, g.cell * 2 - 4, g.cell - 8);
  } else {
    const rr = cellRect(g, b.col, b.row);
    drawSlab(ctx, rr.x + 4, rr.y + 2, g.cell - 8, g.cell * 2 - 4);
  }
}
function drawStanding(ctx, x, y, w, h) {
  ctx.fillStyle = PALETTE.blockEdge;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = PALETTE.block;
  ctx.fillRect(x + 1, y + 1, w - 2, h - 2);
  ctx.fillStyle = PALETTE.blockHi;
  ctx.fillRect(x + 1, y + 1, w - 2, 3);
  ctx.fillRect(x + 1, y + 1, 3, h - 2);
  ctx.fillStyle = PALETTE.blockLo;
  ctx.fillRect(x + w - 4, y + 1, 3, h - 2);
  ctx.fillRect(x + 1, y + h - 4, w - 2, 3);
}
function drawSlab(ctx, x, y, w, h) { drawStanding(ctx, x, y, w, h); }

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves + ' / ' + t(lang, 'par') + ' ' + s.lv.par, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText(t(lang, 'best') + ' ' + (best || '—'), VW - 6, 16);
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
