// Pixel-art rendering for Pixel Untangle. 360x480 world units.

const NODE_R = 11;

const PALETTE = {
  bg:        '#161a2e',
  board:     '#23284a',
  boardEdge: '#39406e',
  grain:     '#2b3158',
  thread:    '#46c2b6',
  threadDk:  '#1d5b56',
  cross:     '#ec5a52',
  crossDk:   '#7e2420',
  threadOk:  '#6fe0a0',
  peg:       '#e6b860',
  pegHi:     '#ffe6a8',
  pegDk:     '#9c7330',
  pegRim:    '#120f1c',
  pegLive:   '#ffd24a',
  hud:       '#0e1124',
  hudText:   '#f3f1e6',
  hudDim:    '#8b93b6',
  good:      '#6fe0a0',
  bad:       '#ff7a72',
  accent:    '#f4c44a',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  // The peg board.
  ctx.fillStyle = PALETTE.board;
  ctx.fillRect(AREA.x0 - 24, AREA.y0 - 26, (AREA.x1 - AREA.x0) + 48, (AREA.y1 - AREA.y0) + 52);
  ctx.fillStyle = PALETTE.grain;
  for (let i = 0; i < 90; i++) {
    const gx = (i * 71 + 19) % ((AREA.x1 - AREA.x0) + 40) + AREA.x0 - 20;
    const gy = (i * 53 + 11) % ((AREA.y1 - AREA.y0) + 44) + AREA.y0 - 22;
    ctx.fillRect(gx, gy, 2, 2);
  }
  ctx.strokeStyle = PALETTE.boardEdge;
  ctx.lineWidth = 2;
  ctx.strokeRect(AREA.x0 - 24, AREA.y0 - 26, (AREA.x1 - AREA.x0) + 48, (AREA.y1 - AREA.y0) + 52);
}

function drawThread(ctx, a, b, mode) {
  // mode: 'ok' | 'cross' | 'solved'
  const dk = mode === 'cross' ? PALETTE.crossDk : PALETTE.threadDk;
  const fg = mode === 'cross' ? PALETTE.cross
           : mode === 'solved' ? PALETTE.threadOk : PALETTE.thread;
  ctx.lineCap = 'round';
  ctx.strokeStyle = dk;
  ctx.lineWidth = 6;
  ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
  ctx.strokeStyle = fg;
  ctx.lineWidth = 3;
  ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
}

function drawGraph(ctx, p, pos, crossSet, dragIdx, solved) {
  // Clean threads first, crossing threads on top.
  for (let i = 0; i < p.edges.length; i++) {
    if (crossSet.has(i)) continue;
    const e = p.edges[i];
    drawThread(ctx, pos[e[0]], pos[e[1]], solved ? 'solved' : 'ok');
  }
  for (let i = 0; i < p.edges.length; i++) {
    if (!crossSet.has(i)) continue;
    const e = p.edges[i];
    drawThread(ctx, pos[e[0]], pos[e[1]], 'cross');
  }
  for (let i = 0; i < p.n; i++) drawPeg(ctx, pos[i].x, pos[i].y, i === dragIdx);
}

function drawPeg(ctx, x, y, live) {
  ctx.fillStyle = PALETTE.pegRim;
  ctx.beginPath(); ctx.arc(x, y, NODE_R + 2, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = live ? PALETTE.pegLive : PALETTE.pegDk;
  ctx.beginPath(); ctx.arc(x, y, NODE_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = live ? PALETTE.pegHi : PALETTE.peg;
  ctx.beginPath(); ctx.arc(x, y, NODE_R - 3, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.pegHi;
  ctx.fillRect((x - NODE_R / 2) | 0, (y - NODE_R / 2) | 0, 3, 3);
}

function drawHud(ctx, lang, p, count, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (p.levelIndex + 1) + ' ' + p.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillStyle = count === 0 ? PALETTE.good : PALETTE.bad;
  ctx.fillText(t(lang, 'crossings') + ' ' + count, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.hudText;
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), VW - 8, 16);
}

// A small tangled graph for the title screen.
function drawTitleArt(ctx, cx, cy) {
  const pts = [
    { x: cx - 60, y: cy - 36 }, { x: cx + 54, y: cy - 44 },
    { x: cx + 64, y: cy + 30 }, { x: cx - 48, y: cy + 40 },
    { x: cx + 8,  y: cy - 6  }, { x: cx - 14, y: cy + 6 },
  ];
  const es = [[0,2],[1,3],[0,1],[2,3],[4,3],[5,1],[0,4],[2,5]];
  for (let i = 0; i < es.length; i++) {
    drawThread(ctx, pts[es[i][0]], pts[es[i][1]], i % 3 === 0 ? 'cross' : 'ok');
  }
  for (const q of pts) drawPeg(ctx, q.x, q.y, false);
}
