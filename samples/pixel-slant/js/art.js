// Pixel-art rendering for Pixel Slant. 360x480 world units.

const PALETTE = {
  bg:       '#15171f',
  panel:    '#242732',
  panelHi:  '#363a4a',
  cell:     '#1c1f29',
  cellEdge: '#2e3340',
  slash:    '#54c9c0',
  slashDk:  '#235e5a',
  loop:     '#ec5a52',
  loopDk:   '#7e2420',
  point:    '#5a6072',
  clueBg:   '#0e0f15',
  clue:     '#e6e3d4',
  clueOk:   '#5fd07a',
  clueBad:  '#ec5a52',
  hud:      '#0c0d12',
  hudText:  '#f3f1e6',
  hudDim:   '#878da2',
  accent:   '#f4c44a',
  good:     '#5fd07a',
};

function gridGeometry(C) {
  const cell = Math.min(46, (300 / C) | 0);
  const span = cell * C;
  const ox = ((VW - span) / 2) | 0;
  const oy = 80;
  return { cell, span, ox, oy };
}

function pointXY(g, r, c) { return { x: g.ox + c * g.cell, y: g.oy + r * g.cell }; }

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawBoard(ctx, puzzle, cells) {
  const C = puzzle.C, g = gridGeometry(C);
  // Panel + cell grid.
  ctx.fillStyle = PALETTE.panel;
  ctx.fillRect(g.ox - 14, g.oy - 14, g.span + 28, g.span + 28);
  ctx.fillStyle = PALETTE.panelHi;
  ctx.fillRect(g.ox - 14, g.oy - 14, g.span + 28, 3);
  for (let r = 0; r < C; r++) {
    for (let c = 0; c < C; c++) {
      ctx.fillStyle = PALETTE.cell;
      ctx.fillRect(g.ox + c * g.cell + 1, g.oy + r * g.cell + 1, g.cell - 2, g.cell - 2);
    }
  }
  ctx.strokeStyle = PALETTE.cellEdge;
  ctx.lineWidth = 1;
  for (let i = 0; i <= C; i++) {
    ctx.beginPath(); ctx.moveTo(g.ox + i * g.cell + 0.5, g.oy); ctx.lineTo(g.ox + i * g.cell + 0.5, g.oy + g.span); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(g.ox, g.oy + i * g.cell + 0.5); ctx.lineTo(g.ox + g.span, g.oy + i * g.cell + 0.5); ctx.stroke();
  }
  // Diagonals - loop ones in red.
  const loop = loopCells(C, cells);
  ctx.lineCap = 'round';
  for (let cell = 0; cell < C * C; cell++) {
    if (!cells[cell]) continue;
    const r = (cell / C) | 0, c = cell % C;
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    let ax, ay, bx, by;
    if (cells[cell] === D_BACK) { ax = x; ay = y; bx = x + g.cell; by = y + g.cell; }
    else                       { ax = x + g.cell; ay = y; bx = x; by = y + g.cell; }
    const onLoop = loop.has(cell);
    ctx.strokeStyle = onLoop ? PALETTE.loopDk : PALETTE.slashDk;
    ctx.lineWidth = 6;
    ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
    ctx.strokeStyle = onLoop ? PALETTE.loop : PALETTE.slash;
    ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
  }
  // Lattice points + clue numbers.
  const count = pointState(C, puzzle.clues, cells);
  ctx.font = 'bold ' + Math.min(15, (g.cell * 0.42) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let r = 0; r <= C; r++) {
    for (let c = 0; c <= C; c++) {
      const p = pointXY(g, r, c);
      const v = r * (C + 1) + c;
      if (puzzle.clues[v] >= 0) {
        ctx.fillStyle = PALETTE.clueBg;
        ctx.beginPath(); ctx.arc(p.x, p.y, 9, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = count[v] === puzzle.clues[v] ? PALETTE.clueOk
                      : (count[v] > puzzle.clues[v] ? PALETTE.clueBad : PALETTE.clue);
        ctx.fillText(String(puzzle.clues[v]), p.x, p.y + 1);
      } else {
        ctx.fillStyle = PALETTE.point;
        ctx.fillRect((p.x - 2) | 0, (p.y - 2) | 0, 4, 4);
      }
    }
  }
}

function drawHud(ctx, lang, puzzle, cells, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (puzzle.levelIndex + 1) + ' ' + puzzle.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  let filled = 0;
  for (let i = 0; i < puzzle.C * puzzle.C; i++) if (cells[i]) filled++;
  ctx.fillText(filled + '/' + (puzzle.C * puzzle.C), VW / 2, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), VW - 8, 16);
}

// A small slashed grid for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const cell = 30, n = 3, ox = cx - cell * n / 2, oy = cy - cell * n / 2;
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) {
    ctx.fillStyle = PALETTE.cell;
    ctx.fillRect(ox + c * cell + 1, oy + r * cell + 1, cell - 2, cell - 2);
  }
  const pat = [D_BACK, D_FWD, D_BACK, D_FWD, D_BACK, D_FWD, D_FWD, D_BACK, D_FWD];
  ctx.lineCap = 'round'; ctx.strokeStyle = PALETTE.slash; ctx.lineWidth = 4;
  for (let i = 0; i < 9; i++) {
    const r = (i / n) | 0, c = i % n, x = ox + c * cell, y = oy + r * cell;
    ctx.beginPath();
    if (pat[i] === D_BACK) { ctx.moveTo(x, y); ctx.lineTo(x + cell, y + cell); }
    else                   { ctx.moveTo(x + cell, y); ctx.lineTo(x, y + cell); }
    ctx.stroke();
  }
  ctx.fillStyle = PALETTE.point;
  for (let r = 0; r <= n; r++) for (let c = 0; c <= n; c++)
    ctx.fillRect(ox + c * cell - 2, oy + r * cell - 2, 4, 4);
}
