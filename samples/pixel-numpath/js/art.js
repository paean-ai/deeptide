// Pixel-art rendering for Pixel Numpath. 360x480 world units.

const PALETTE = {
  bg:       '#141826',
  panel:    '#222a40',
  panelHi:  '#33405c',
  cell:     '#2c3650',
  cellEdge: '#3e4c6e',
  pathCell: '#2a5a86',
  pathLine: '#46b8e8',
  pathEnd:  '#7fe0a0',
  clue:     '#f4c44a',
  clueCell: '#3a3650',
  step:     '#bfe6ff',
  ink:      '#10131f',
  hud:      '#0e1018',
  hudText:  '#f3f1e6',
  hudDim:   '#8b93b0',
  accent:   '#f4c44a',
  good:     '#7fe0a0',
};

function gridGeometry(C) {
  const cell = Math.min(48, (300 / C) | 0);
  const span = cell * C;
  const ox = ((VW - span) / 2) | 0;
  const oy = 84;
  return { cell, span, ox, oy };
}

function cellCenter(g, C, cell) {
  return { x: g.ox + (cell % C) * g.cell + g.cell / 2,
           y: g.oy + ((cell / C) | 0) * g.cell + g.cell / 2 };
}

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawBoard(ctx, puzzle, path) {
  const C = puzzle.C, g = gridGeometry(C);
  const stepOf = new Array(C * C).fill(-1);
  for (let k = 0; k < path.length; k++) stepOf[path[k]] = k;
  // Panel.
  ctx.fillStyle = PALETTE.panel;
  ctx.fillRect(g.ox - 14, g.oy - 14, g.span + 28, g.span + 28);
  ctx.fillStyle = PALETTE.panelHi;
  ctx.fillRect(g.ox - 14, g.oy - 14, g.span + 28, 3);
  // Cells.
  for (let i = 0; i < C * C; i++) {
    const x = g.ox + (i % C) * g.cell, y = g.oy + ((i / C) | 0) * g.cell;
    const inPath = stepOf[i] >= 0;
    ctx.fillStyle = inPath ? PALETTE.pathCell
                  : (puzzle.clues[i] ? PALETTE.clueCell : PALETTE.cell);
    ctx.fillRect(x + 1, y + 1, g.cell - 2, g.cell - 2);
    ctx.fillStyle = PALETTE.cellEdge;
    ctx.fillRect(x + 1, y + 1, g.cell - 2, 1);
  }
  // The path as a connecting snake.
  if (path.length > 1) {
    ctx.strokeStyle = PALETTE.pathLine;
    ctx.lineWidth = Math.max(4, g.cell * 0.26);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let k = 0; k < path.length; k++) {
      const c = cellCenter(g, C, path[k]);
      if (k === 0) ctx.moveTo(c.x, c.y); else ctx.lineTo(c.x, c.y);
    }
    ctx.stroke();
  }
  // The path end marker.
  if (path.length) {
    const e = cellCenter(g, C, path[path.length - 1]);
    ctx.fillStyle = PALETTE.pathEnd;
    ctx.fillRect((e.x - 5) | 0, (e.y - 5) | 0, 10, 10);
  }
  // Numbers: givens in gold, filled steps in pale blue.
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const fs = Math.min(17, (g.cell * 0.44) | 0);
  for (let i = 0; i < C * C; i++) {
    const given = puzzle.clues[i];
    const num = given ? given : (stepOf[i] >= 0 ? stepOf[i] + 1 : 0);
    if (!num) continue;
    const c = cellCenter(g, C, i);
    ctx.font = 'bold ' + fs + 'px monospace';
    ctx.fillStyle = given ? PALETTE.clue : PALETTE.step;
    ctx.fillText(String(num), c.x, c.y + 1);
  }
}

function drawHud(ctx, lang, puzzle, path, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (puzzle.levelIndex + 1) + ' ' + puzzle.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText(path.length + '/' + (puzzle.C * puzzle.C), VW / 2, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), VW - 8, 16);
}

// A small numbered path for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const cell = 30, ox = cx - cell * 2, oy = cy - cell * 1.5;
  const cells = [[0,0],[1,0],[2,0],[3,0],[3,1],[2,1],[1,1],[0,1],[0,2],[1,2],[2,2],[3,2]];
  for (let r = 0; r <= 2; r++) for (let c = 0; c <= 3; c++) {
    ctx.fillStyle = PALETTE.pathCell;
    ctx.fillRect(ox + c * cell + 1, oy + r * cell + 1, cell - 2, cell - 2);
  }
  ctx.strokeStyle = PALETTE.pathLine;
  ctx.lineWidth = 7; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.beginPath();
  cells.forEach(([c, r], k) => {
    const x = ox + c * cell + cell / 2, y = oy + r * cell + cell / 2;
    if (k === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  });
  ctx.stroke();
  ctx.fillStyle = PALETTE.clue;
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('1', ox + cell / 2, oy + cell / 2 + 1);
  ctx.fillText('12', ox + 3 * cell + cell / 2, oy + 2 * cell + cell / 2 + 1);
}
