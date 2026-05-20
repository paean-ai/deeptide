// Pixel-art rendering for Pixel Peg Jump. 360x480 world units.

const PALETTE = {
  bg:        '#10142a',
  cardLite:  '#1f2748',
  cardDark:  '#161b36',
  board:     '#262f54',
  boardEdge: '#0a0d20',
  hole:      '#0d1126',
  holeHi:    '#1b2244',
  peg:       '#f4d27b',
  pegHi:     '#fcf0c4',
  pegLo:     '#8c6a2c',
  pegSel:    '#f97648',
  pegSelHi:  '#ffe1c0',
  pegTgt:    '#5fc06e',
  pegTgtHi:  '#bce8b8',
  border:    '#0a0d20',
  hud:       '#0a0d20',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  star:      '#f8d34a',
  starOff:   '#3a4274',
};

// Compute cell size so the board fits comfortably between the HUD and
// the action bar. Boards range from 3..7 wide / 3..7 tall.
function gridGeometry(w, h) {
  const maxBoard = Math.max(w, h);
  const cell = maxBoard <= 4 ? 56 : maxBoard <= 5 ? 48 : maxBoard <= 6 ? 42 : 38;
  const bw = cell * w, bh = cell * h;
  const ox = ((360 - bw) / 2) | 0;
  const oy = (60 + (380 - bh) / 2) | 0;
  return { cell, bw, bh, ox, oy };
}
function cellRect(w, h, x, y) {
  const g = gridGeometry(w, h);
  return { x: g.ox + x * g.cell, y: g.oy + y * g.cell, w: g.cell, h: g.cell };
}

function drawBackdrop(ctx) {
  // Soft vertical gradient + dotted pixel border around the playfield.
  const grad = ctx.createLinearGradient(0, 0, 0, 480);
  grad.addColorStop(0, '#0c1024');
  grad.addColorStop(1, '#161b36');
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 360, 480);
  // Random-but-stable pixel speckle for texture.
  ctx.fillStyle = '#1c2240';
  for (let i = 0; i < 36; i++) {
    const sx = (i * 47 + 3) % 360;
    const sy = (i * 73 + 11) % 480;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s, suggested) {
  const g = gridGeometry(s.w, s.h);
  // Card framing.
  const pad = 6;
  ctx.fillStyle = PALETTE.cardDark;
  ctx.fillRect(g.ox - pad - 1, g.oy - pad - 1, g.bw + pad * 2 + 2, g.bh + pad * 2 + 2);
  ctx.fillStyle = PALETTE.cardLite;
  ctx.fillRect(g.ox - pad, g.oy - pad, g.bw + pad * 2, g.bh + pad * 2);
  // Per-cell.
  for (let y = 0; y < s.h; y++) for (let x = 0; x < s.w; x++) {
    const v = s.grid[y][x];
    if (v === ' ') continue;
    const r = cellRect(s.w, s.h, x, y);
    // Cell tile (board square)
    ctx.fillStyle = PALETTE.boardEdge;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = PALETTE.board;
    ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    // Hole socket
    const cx = r.x + r.w / 2, cy = r.y + r.h / 2;
    const rad = (r.w * 0.42) | 0;
    ctx.fillStyle = PALETTE.hole;
    drawDisk(ctx, cx, cy, rad);
    ctx.fillStyle = PALETTE.holeHi;
    drawDisk(ctx, cx - 1, cy - 1, rad - 2);
    ctx.fillStyle = PALETTE.hole;
    drawDisk(ctx, cx, cy, rad - 3);
    if (v === 'O') drawPeg(ctx, cx, cy, rad - 1, false, false);
    if (s.sel && s.sel[0] === x && s.sel[1] === y) drawPeg(ctx, cx, cy, rad - 1, true, false);
    if (suggested && suggested.has(x + ',' + y)) drawTarget(ctx, cx, cy, rad - 2);
  }
}

// Filled disk drawn as a pixel-art chunky circle (no anti-aliasing reliance).
function drawDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const dxMax = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - dxMax) | 0, (cy + dy) | 0, dxMax * 2 + 1, 1);
  }
}
function drawPeg(ctx, cx, cy, r, selected, target) {
  ctx.fillStyle = selected ? PALETTE.pegSel : PALETTE.peg;
  drawDisk(ctx, cx, cy, r);
  ctx.fillStyle = selected ? PALETTE.pegSelHi : PALETTE.pegHi;
  drawDisk(ctx, cx - 1, cy - 1, Math.max(1, r - 2));
  if (!selected) {
    ctx.fillStyle = PALETTE.pegLo;
    drawDisk(ctx, cx + 2, cy + 2, Math.max(1, r - 4));
    ctx.fillStyle = PALETTE.peg;
    drawDisk(ctx, cx + 1, cy + 1, Math.max(1, r - 5));
  }
}
function drawTarget(ctx, cx, cy, r) {
  // Hollow green ring marks legal landing spots.
  ctx.strokeStyle = PALETTE.pegTgt;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.arc(cx, cy, Math.max(2, r - 1), 0, Math.PI * 2);
  ctx.stroke();
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 8, 16);
  ctx.textAlign = 'center';
  const pegs = pegCount(s);
  ctx.fillText(t(lang, 'pegsLeft') + ' ' + pegs, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'moves') + ' ' + s.history.length, 352, 16);
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
    const px = cx + Math.cos(a) * rr;
    const py = cy + Math.sin(a) * rr;
    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
  }
  ctx.closePath();
  ctx.fill();
}
