// Pixel-art rendering for Pixel Edgematch. 360x480 world units.

const PALETTE = {
  bg:       '#1b1d28',
  panel:    '#282b3a',
  panelHi:  '#3a3e52',
  tileEdge: '#10121a',
  center:   '#1b1d28',
  sel:      '#f4e0a0',
  mismatch: '#ff5a52',
  hud:      '#0e0f16',
  hudText:  '#f3f1e6',
  hudDim:   '#888ea4',
  accent:   '#f4c44a',
  good:     '#5fc06e',
};
// Edge colours: 0 = grey border, 1..5 puzzle colours.
const EDGE_COLORS = ['#5a5f6e', '#e8554f', '#4aa6e0', '#f4c44a', '#5fc06e', '#a06fd0'];
const EDGE_HI     = ['#7a8092', '#ff8a82', '#7fcdf4', '#ffe08a', '#8fe09a', '#c79be8'];

function gridGeometry(w, h) {
  const cell = Math.min(56, (300 / w) | 0, (250 / h) | 0);
  return { cell, ox: ((VW - cell * w) / 2) | 0, oy: 60, totalW: cell * w, totalH: cell * h };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawTile(ctx, x, y, cell, tile, selected) {
  const cx = x + cell / 2, cy = y + cell / 2;
  const cols = [
    [[x, y], [x + cell, y]],                    // TOP
    [[x + cell, y], [x + cell, y + cell]],      // RIGHT
    [[x + cell, y + cell], [x, y + cell]],      // BOTTOM
    [[x, y + cell], [x, y]],                    // LEFT
  ];
  for (let d = 0; d < 4; d++) {
    const col = edgeOf(tile, d);
    ctx.fillStyle = EDGE_COLORS[col] || '#888';
    ctx.beginPath();
    ctx.moveTo(cols[d][0][0], cols[d][0][1]);
    ctx.lineTo(cols[d][1][0], cols[d][1][1]);
    ctx.lineTo(cx, cy);
    ctx.closePath();
    ctx.fill();
    // a thin highlight along the outer edge
    ctx.strokeStyle = EDGE_HI[col] || '#aaa';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(cols[d][0][0], cols[d][0][1]);
    ctx.lineTo(cols[d][1][0], cols[d][1][1]);
    ctx.stroke();
  }
  ctx.fillStyle = PALETTE.center;
  ctx.fillRect((cx - 3) | 0, (cy - 3) | 0, 6, 6);
  ctx.strokeStyle = selected ? PALETTE.sel : PALETTE.tileEdge;
  ctx.lineWidth = selected ? 3 : 1;
  ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
}

function drawBoard(ctx, s) {
  const g = gridGeometry(s.w, s.h);
  ctx.fillStyle = PALETTE.panel;
  ctx.fillRect(g.ox - 6, g.oy - 6, g.totalW + 12, g.totalH + 12);
  ctx.fillStyle = PALETTE.panelHi;
  ctx.fillRect(g.ox - 6, g.oy - 6, g.totalW + 12, 3);
  for (let r = 0; r < s.h; r++) {
    for (let c = 0; c < s.w; c++) {
      const cell = r * s.w + c;
      drawTile(ctx, g.ox + c * g.cell, g.oy + r * g.cell, g.cell, tileAt(s, cell), s.selected === cell);
    }
  }
  // Mismatched shared edges marked red.
  for (const e of edgeReport(s)) {
    if (e.ok) continue;
    const x = g.ox + e.c * g.cell, y = g.oy + e.r * g.cell;
    ctx.fillStyle = PALETTE.mismatch;
    if (e.side === 'right')  ctx.fillRect(x + g.cell - 2, y + 4, 4, g.cell - 8);
    else                     ctx.fillRect(x + 4, y + g.cell - 2, g.cell - 8, 4);
  }
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 30);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 15);
  ctx.textAlign = 'center';
  const er = edgeReport(s);
  ctx.fillText(matchedCount(s) + '/' + er.length, VW / 2, 15);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves, VW - 8, 15);
}

// A 2x2 of edge tiles for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const cell = 46, ox = cx - cell, oy = cy - cell;
  const demo = [
    { base: [0, 1, 2, 0], rot: 0 }, { base: [0, 0, 3, 1], rot: 0 },
    { base: [2, 4, 0, 0], rot: 0 }, { base: [3, 0, 0, 4], rot: 0 },
  ];
  for (let i = 0; i < 4; i++) {
    drawTile(ctx, ox + (i % 2) * cell, oy + ((i / 2) | 0) * cell, cell, demo[i], false);
  }
}
