// Pixel-art rendering for Pixel Fit. 360x480 world units.

const PALETTE = {
  bg:       '#1c2030',
  bgLo:     '#141826',
  frame:    '#2e3550',
  slot:     '#10131f',
  slotEdge: '#3a4060',
  grout:    '#10131f',
  tray:     '#262b40',
  trayEdge: '#3a4060',
  sel:      '#f4c44a',
  hud:      '#0e1018',
  hudText:  '#f3f1e6',
  hudDim:   '#8a90a8',
  accent:   '#f4c44a',
  good:     '#5fc06e',
};

function frameGeo(s) {
  const cell = Math.min(46, (300 / s.w) | 0, (244 / s.h) | 0);
  const ox = ((VW - cell * s.w) / 2) | 0;
  const oy = 46;
  return { cell, ox, oy, totalW: cell * s.w, totalH: cell * s.h };
}

// Tray slot for the i-th tray (unplaced) piece: a 5-column grid below the frame.
function traySlot(s, i) {
  const g = frameGeo(s);
  const top = g.oy + g.totalH + 12;
  const slotW = 68, slotH = Math.min(94, ((472 - top) / 2) | 0);
  const col = i % 5, row = (i / 5) | 0;
  return { x: 10 + col * slotW, y: top + row * slotH, w: slotW, h: slotH };
}

function shade(hex, f) {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.max(0, Math.min(255, ((n >> 16) & 255) * f)) | 0;
  const g = Math.max(0, Math.min(255, ((n >> 8) & 255) * f)) | 0;
  const b = Math.max(0, Math.min(255, (n & 255) * f)) | 0;
  return 'rgb(' + r + ',' + g + ',' + b + ')';
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgLo;
  for (let i = 0; i < 60; i++) {
    ctx.fillRect((i * 71 + 13) % VW, (i * 53 + 29) % VH, 2, 2);
  }
}

function drawBlock(ctx, x, y, cell, color) {
  ctx.fillStyle = shade(color, 0.6);
  ctx.fillRect(x, y, cell, cell);
  ctx.fillStyle = color;
  ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
  ctx.fillStyle = shade(color, 1.35);
  ctx.fillRect(x + 2, y + 2, cell - 4, 2);
}

function drawFrame(ctx, s) {
  const g = frameGeo(s);
  // Frame backing.
  ctx.fillStyle = PALETTE.frame;
  ctx.fillRect(g.ox - 5, g.oy - 5, g.totalW + 10, g.totalH + 10);
  ctx.fillStyle = PALETTE.slotEdge;
  ctx.fillRect(g.ox - 5, g.oy - 5, g.totalW + 10, 2);
  for (let r = 0; r < s.h; r++) {
    for (let c = 0; c < s.w; c++) {
      const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
      const owner = s.occ[r * s.w + c];
      if (owner === -1) {
        ctx.fillStyle = PALETTE.slot;
        ctx.fillRect(x + 1, y + 1, g.cell - 2, g.cell - 2);
        ctx.fillStyle = PALETTE.slotEdge;
        ctx.fillRect(x + 3, y + 3, g.cell - 6, 1);
      } else {
        drawBlock(ctx, x, y, g.cell, s.pieces[owner].color);
      }
    }
  }
}

function drawTray(ctx, s, lang) {
  let slotIndex = 0;
  for (let i = 0; i < s.pieces.length; i++) {
    const p = s.pieces[i];
    if (p.placed) continue;
    const slot = traySlot(s, slotIndex++);
    const selected = s.selected === i;
    ctx.fillStyle = selected ? shade(PALETTE.sel, 0.4) : PALETTE.tray;
    ctx.fillRect(slot.x + 2, slot.y + 2, slot.w - 4, slot.h - 4);
    ctx.strokeStyle = selected ? PALETTE.sel : PALETTE.trayEdge;
    ctx.lineWidth = selected ? 2 : 1;
    ctx.strokeRect(slot.x + 2.5, slot.y + 2.5, slot.w - 5, slot.h - 5);
    drawPieceMini(ctx, p, slot.x + slot.w / 2, slot.y + slot.h / 2, selected);
  }
}

function drawPieceMini(ctx, piece, cx, cy, big) {
  let maxX = 0, maxY = 0;
  for (const [x, y] of piece.cells) { if (x > maxX) maxX = x; if (y > maxY) maxY = y; }
  const mc = big ? 13 : 11;
  const ox = cx - ((maxX + 1) * mc) / 2;
  const oy = cy - ((maxY + 1) * mc) / 2;
  for (const [x, y] of piece.cells) {
    drawBlock(ctx, (ox + x * mc) | 0, (oy + y * mc) | 0, mc, piece.color);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 30);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 66, 15);
  ctx.textAlign = 'center';
  const placed = s.pieces.filter(p => p.placed).length;
  ctx.fillText(placed + '/' + s.pieces.length, VW / 2, 15);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves, VW - 8, 15);
}

// A neat little assembled frame for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const demo = [
    { cells: [[0,0],[1,0],[0,1]],       color: '#e8554f' },
    { cells: [[2,0],[2,1],[1,1]],       color: '#46b8c4' },
    { cells: [[0,2],[1,2],[2,2]],       color: '#f4c84a' },
  ];
  const cell = 22, ox = cx - cell * 1.5, oy = cy - cell * 1.5;
  ctx.fillStyle = PALETTE.frame;
  ctx.fillRect(ox - 4, oy - 4, cell * 3 + 8, cell * 3 + 8);
  for (const p of demo) {
    for (const [x, y] of p.cells) {
      drawBlock(ctx, ox + x * cell, oy + y * cell, cell, p.color);
    }
  }
}
