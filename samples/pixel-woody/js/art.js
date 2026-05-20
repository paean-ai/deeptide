// Pixel-art rendering for Pixel Woody. 360x480 world units.

const PALETTE = {
  bg:        '#1a120a',
  bgHi:      '#241a10',
  boardBg:   '#2a1d10',
  cellEmpty: '#3a2b1c',
  cellEmptyHi: '#4c3a26',
  cellEdge:  '#0e0805',
  trayBg:    '#241a10',
  trayEdge:  '#0e0805',
  select:    '#ffd34a',
  hud:       '#0e0805',
  hudText:   '#fbf3e2',
  hudDim:    '#a08c70',
  win:       '#5fc06e',
  star:      '#f8d34a',
  starOff:   '#3a2b1c',
};

// Board: 8 x 8 cells of 36 px = 288 px. Centred at x=36; top sits below HUD.
const CELL = 36;
const BOARD_OX = 36;
const BOARD_OY = 48;
const BOARD_PX = GRID * CELL;

// Tray: three slots beneath the board.
const TRAY_OY = BOARD_OY + BOARD_PX + 18;
const TRAY_CELL = 18;             // size of one tray-piece cell (smaller than board)
const TRAY_SLOT_W = 96;

function trayRect(i) {
  // Three evenly spaced slots across the bottom.
  const totalW = TRAY_SIZE * TRAY_SLOT_W + (TRAY_SIZE - 1) * 12;
  const x0 = ((VW - totalW) / 2) | 0;
  return { x: x0 + i * (TRAY_SLOT_W + 12), y: TRAY_OY, w: TRAY_SLOT_W, h: 56 };
}

function cellRect(c, r) {
  return { x: BOARD_OX + c * CELL, y: BOARD_OY + r * CELL, w: CELL, h: CELL };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // Faint wood-grain speckle.
  ctx.fillStyle = PALETTE.bgHi;
  for (let i = 0; i < 28; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawBoard(ctx, s, ghost) {
  // Board frame.
  ctx.fillStyle = PALETTE.cellEdge;
  ctx.fillRect(BOARD_OX - 3, BOARD_OY - 3, BOARD_PX + 6, BOARD_PX + 6);
  ctx.fillStyle = PALETTE.boardBg;
  ctx.fillRect(BOARD_OX, BOARD_OY, BOARD_PX, BOARD_PX);
  for (let y = 0; y < GRID; y++) for (let x = 0; x < GRID; x++) {
    const r = cellRect(x, y);
    const v = s.grid[y][x];
    ctx.fillStyle = PALETTE.cellEdge;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    if (v === null) {
      ctx.fillStyle = PALETTE.cellEmpty;
      ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
      ctx.fillStyle = PALETTE.cellEmptyHi;
      ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, 1);
    } else {
      drawBlockCell(ctx, r.x, r.y, r.w, r.h, v);
    }
  }
  // Ghost preview of where a selected piece will land.
  if (ghost) {
    for (const [cx, cy] of ghost.cells) {
      const r = cellRect(cx, cy);
      ctx.fillStyle = ghost.valid ? 'rgba(95,192,110,0.35)' : 'rgba(255,90,90,0.35)';
      ctx.fillRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
    }
  }
}

function drawBlockCell(ctx, x, y, w, h, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x + 1, y + 1, w - 2, h - 2);
  // Lighten top + left, darken bottom + right.
  ctx.fillStyle = lighten(color, 0.22);
  ctx.fillRect(x + 1, y + 1, w - 2, 3);
  ctx.fillRect(x + 1, y + 1, 3, h - 2);
  ctx.fillStyle = darken(color, 0.30);
  ctx.fillRect(x + 1, y + h - 4, w - 2, 3);
  ctx.fillRect(x + w - 4, y + 1, 3, h - 2);
}
function lighten(hex, amt) { return _tint(hex, amt); }
function darken(hex, amt)  { return _tint(hex, -amt); }
function _tint(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 0xff, g = (n >> 8) & 0xff, b = n & 0xff;
  r = Math.max(0, Math.min(255, Math.round(r + 255 * amt)));
  g = Math.max(0, Math.min(255, Math.round(g + 255 * amt)));
  b = Math.max(0, Math.min(255, Math.round(b + 255 * amt)));
  return '#' + [r, g, b].map(v => v.toString(16).padStart(2, '0')).join('');
}

function drawTray(ctx, s) {
  for (let i = 0; i < TRAY_SIZE; i++) {
    const slot = trayRect(i);
    ctx.fillStyle = PALETTE.trayEdge;
    ctx.fillRect(slot.x - 1, slot.y - 1, slot.w + 2, slot.h + 2);
    ctx.fillStyle = PALETTE.trayBg;
    ctx.fillRect(slot.x, slot.y, slot.w, slot.h);
    if (i >= s.tray.length) continue;
    const piece = s.tray[i];
    // Centre the piece in its slot.
    let maxX = 0, maxY = 0;
    for (const [cx, cy] of piece.cells) { if (cx > maxX) maxX = cx; if (cy > maxY) maxY = cy; }
    const pw = (maxX + 1) * TRAY_CELL, ph = (maxY + 1) * TRAY_CELL;
    const ox = slot.x + ((slot.w - pw) / 2) | 0;
    const oy = slot.y + ((slot.h - ph) / 2) | 0;
    for (const [cx, cy] of piece.cells) {
      drawBlockCell(ctx, ox + cx * TRAY_CELL, oy + cy * TRAY_CELL, TRAY_CELL, TRAY_CELL, piece.color);
    }
    if (s.selected === i) {
      ctx.strokeStyle = PALETTE.select;
      ctx.lineWidth = 2;
      ctx.strokeRect(slot.x + 1, slot.y + 1, slot.w - 2, slot.h - 2);
    }
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'placed') + ' ' + s.placed, VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText(t(lang, 'high') + ' ' + (best || 0), VW - 6, 16);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.4);
  ctx.fillStyle = `rgba(255,221,90,${0.35 * a})`;
  ctx.fillRect(BOARD_OX, BOARD_OY, BOARD_PX, BOARD_PX);
}
