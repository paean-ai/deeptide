// Pixel-art rendering for Pixel Knight. 360x480 world units.

const PALETTE = {
  bg:        '#1a1224',
  bgHi:      '#28203a',
  light:     '#f0d6a8',          // chessboard light square
  dark:      '#7a4a1f',          // chessboard dark square
  edge:      '#070315',
  visited:   'rgba(95,192,110,0.42)',
  visitedDot:'#5fc06e',
  knightBg:  '#5fc0ff',
  knightHi:  '#a8e0ff',
  knightLo:  '#205a8a',
  knightEye: '#0a0a18',
  target:    'rgba(95,192,255,0.42)',
  targetRing:'#5fc0ff',
  hint:      'rgba(255,143,208,0.55)',
  hintRing:  '#ff8fd0',
  digit:     '#0a0a18',
  digitHi:   '#fff7ed',
  hud:       '#070315',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  star:      '#f8d34a',
  starOff:   '#3a4274',
  win:       '#5fc06e',
};

// Geometry: board is centered. Cell size adapts to (n): 5 -> 56, 6 -> 48,
// 7 -> 42, 8 -> 36 — always within 320 px width.
function gridGeometry(n) {
  const cell = Math.min(60, Math.floor(320 / n));
  const bw = cell * n;
  const ox = ((360 - bw) / 2) | 0;
  const oy = 60;
  return { cell, bw, ox, oy };
}

function cellRect(g, c, r) {
  return { x: g.ox + c * g.cell, y: g.oy + r * g.cell, w: g.cell, h: g.cell };
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
  const g = gridGeometry(s.n);
  // Outer frame.
  ctx.fillStyle = PALETTE.edge;
  ctx.fillRect(g.ox - 4, g.oy - 4, g.bw + 8, g.bw + 8);
  // Chessboard pattern.
  for (let r = 0; r < s.n; r++) for (let c = 0; c < s.n; c++) {
    const rect = cellRect(g, c, r);
    ctx.fillStyle = ((c + r) % 2 === 0) ? PALETTE.light : PALETTE.dark;
    ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
    const v = s.visited[r][c];
    if (v) {
      ctx.fillStyle = PALETTE.visited;
      ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
      // Move number, centred.
      ctx.fillStyle = (c + r) % 2 === 0 ? PALETTE.digit : PALETTE.digitHi;
      ctx.font = 'bold ' + (g.cell * 0.42 | 0) + 'px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(v), rect.x + rect.w / 2, rect.y + rect.h / 2 + 1);
    }
  }
}

function drawTargets(ctx, s, targets, hints) {
  if (s.over) return;
  const g = gridGeometry(s.n);
  for (const [tx, ty] of targets) {
    const isHint = hints.some(h => h[0] === tx && h[1] === ty);
    const r = cellRect(g, tx, ty);
    ctx.fillStyle = isHint ? PALETTE.hint : PALETTE.target;
    ctx.fillRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
    ctx.strokeStyle = isHint ? PALETTE.hintRing : PALETTE.targetRing;
    ctx.lineWidth = 2;
    ctx.strokeRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
    if (isHint) {
      // Small dot in the centre to mark Warnsdorff pick.
      ctx.fillStyle = PALETTE.hintRing;
      ctx.fillRect(r.x + r.w / 2 - 2, r.y + r.h / 2 - 2, 4, 4);
    }
  }
}

function drawKnight(ctx, s) {
  const g = gridGeometry(s.n);
  const r = cellRect(g, s.cx, s.cy);
  const cx = r.x + r.w / 2, cy = r.y + r.h / 2;
  ctx.fillStyle = PALETTE.knightLo;
  fillDisk(ctx, cx, cy + 1, r.w * 0.34);
  ctx.fillStyle = PALETTE.knightBg;
  fillDisk(ctx, cx, cy, r.w * 0.32);
  ctx.fillStyle = PALETTE.knightHi;
  fillDisk(ctx, cx - 2, cy - 2, r.w * 0.16);
  // Knight head silhouette: a small "N" letter inside.
  ctx.fillStyle = PALETTE.knightEye;
  ctx.font = 'bold ' + (g.cell * 0.36 | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('N', cx, cy + 1);
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
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'visited') + ' ' + s.moves + '/' + (s.n * s.n), VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText('best ' + (best || '—'), VW - 6, 16);
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
