// Pixel-art rendering for Pixel Slitherlink. 360x480 world units.

const PALETTE = {
  bg:       '#16181f',
  panel:    '#23262f',
  panelHi:  '#333743',
  dot:      '#5a5f70',
  dotLoop:  '#f4e2a0',
  line:     '#f0c64a',
  lineWin:  '#7fe08a',
  cross:    '#5a5f70',
  clue:     '#e6e3d4',
  clueDone: '#5b6072',
  clueBad:  '#ec5a52',
  hud:      '#0e0f15',
  hudText:  '#f3f1e6',
  hudDim:   '#8b90a4',
  accent:   '#f4c44a',
  good:     '#7fe08a',
};

function gridGeometry(C) {
  const cell = Math.min(46, (300 / C) | 0);
  const span = cell * C;
  const ox = ((VW - span) / 2) | 0;
  const oy = 84;
  return { cell, span, ox, oy };
}

function vertXY(g, r, c) { return { x: g.ox + c * g.cell, y: g.oy + r * g.cell }; }

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
}

function drawBoard(ctx, puzzle, edgeState, lang, solved) {
  const C = puzzle.C, G = puzzle.graph, g = gridGeometry(C);
  // Panel.
  ctx.fillStyle = PALETTE.panel;
  ctx.fillRect(g.ox - 16, g.oy - 16, g.span + 32, g.span + 32);
  ctx.fillStyle = PALETTE.panelHi;
  ctx.fillRect(g.ox - 16, g.oy - 16, g.span + 32, 3);
  // Clue numbers.
  const bad = clueViolations(puzzle, edgeState);
  ctx.font = 'bold ' + Math.min(20, (g.cell * 0.5) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 0; i < C * C; i++) {
    if (puzzle.clues[i] < 0) continue;
    const x = i % C, y = (i / C) | 0;
    const cx = g.ox + (x + 0.5) * g.cell, cy = g.oy + (y + 0.5) * g.cell;
    const n = clueCount(puzzle, edgeState, i);
    ctx.fillStyle = bad.has(i) ? PALETTE.clueBad
                  : (n === puzzle.clues[i] ? PALETTE.clueDone : PALETTE.clue);
    ctx.fillText(String(puzzle.clues[i]), cx, cy + 1);
  }
  // Edges: lines and crosses.
  for (let e = 0; e < G.edges.length; e++) {
    const ed = G.edges[e];
    const a = vertXY(g, ed.r, ed.c);
    if (edgeState[e] === E_LINE) {
      ctx.fillStyle = solved ? PALETTE.lineWin : PALETTE.line;
      if (ed.kind === 'h') ctx.fillRect(a.x, a.y - 2, g.cell, 4);
      else                 ctx.fillRect(a.x - 2, a.y, 4, g.cell);
    } else if (edgeState[e] === E_CROSS) {
      const mx = ed.kind === 'h' ? a.x + g.cell / 2 : a.x;
      const my = ed.kind === 'h' ? a.y : a.y + g.cell / 2;
      ctx.strokeStyle = PALETTE.cross;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(mx - 3, my - 3); ctx.lineTo(mx + 3, my + 3);
      ctx.moveTo(mx + 3, my - 3); ctx.lineTo(mx - 3, my + 3);
      ctx.stroke();
    }
  }
  // Lattice dots, brighter where the loop passes.
  const deg = new Array(G.nVerts).fill(0);
  for (let e = 0; e < G.edges.length; e++) {
    if (edgeState[e] !== E_LINE) continue;
    deg[G.edges[e].v1]++; deg[G.edges[e].v2]++;
  }
  for (let r = 0; r <= C; r++) for (let c = 0; c <= C; c++) {
    const v = vertXY(g, r, c);
    const lit = deg[r * (C + 1) + c] > 0;
    ctx.fillStyle = lit ? (solved ? PALETTE.lineWin : PALETTE.dotLoop) : PALETTE.dot;
    const s = lit ? 4 : 3;
    ctx.fillRect((v.x - s / 2) | 0, (v.y - s / 2) | 0, s, s);
  }
}

function drawHud(ctx, lang, puzzle, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (puzzle.levelIndex + 1) + ' ' + puzzle.cfg.name[0], 8, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), VW - 8, 16);
}

// A small fixed loop for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const cell = 30, ox = cx - cell * 1.5, oy = cy - cell * 1.5;
  // dots
  ctx.fillStyle = PALETTE.dot;
  for (let r = 0; r <= 3; r++) for (let c = 0; c <= 3; c++) {
    ctx.fillRect(ox + c * cell - 2, oy + r * cell - 2, 4, 4);
  }
  // a loop around the 3x3
  ctx.fillStyle = PALETTE.line;
  ctx.fillRect(ox, oy - 2, cell * 3, 4);
  ctx.fillRect(ox, oy + cell * 3 - 2, cell * 3, 4);
  ctx.fillRect(ox - 2, oy, 4, cell * 3);
  ctx.fillRect(ox + cell * 3 - 2, oy, 4, cell * 3);
  ctx.fillStyle = PALETTE.clue;
  ctx.font = 'bold 15px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (const [r, c, n] of [[0,1,'2'],[1,1,'0'],[2,0,'3']]) {
    ctx.fillText(n, ox + (c + 0.5) * cell, oy + (r + 0.5) * cell + 1);
  }
}
