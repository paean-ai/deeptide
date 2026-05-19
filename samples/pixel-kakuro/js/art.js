// Pixel Kakuro - rendering.

const COL = {
  wall: '#10131f', wallEdge: '#272d44',
  white: '#e9edf7', whiteSel: '#ffe9a8', whiteBad: '#ff9caa',
  digit: '#1a1f33', clue: '#aeb8d4', accent: '#f2c14e',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1b2034');
  g.addColorStop(1, '#0d101c');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(pz) {
  const cell = Math.min(46, Math.floor(312 / pz.w), Math.floor(248 / pz.h));
  return {
    cell,
    gx: Math.round((VW - cell * pz.w) / 2),
    gy: Math.round(96 + (250 - cell * pz.h) / 2),
  };
}

function drawBoard(ctx, pz, geom, st) {
  const { cell, gx, gy } = geom;
  for (let r = 0; r < pz.h; r++) {
    for (let c = 0; c < pz.w; c++) {
      const x = gx + c * cell, y = gy + r * cell;
      if (pz.white[r][c]) drawWhite(ctx, x, y, cell, r, c, pz, st);
      else drawWall(ctx, x, y, cell, r, c, pz);
    }
  }
}

function drawWall(ctx, x, y, cell, r, c, pz) {
  ctx.fillStyle = COL.wall;
  ctx.fillRect(x, y, cell, cell);
  ctx.strokeStyle = COL.wallEdge;
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
  const clue = pz.clues[r + ',' + c];
  if (!clue) return;
  // diagonal divider
  ctx.strokeStyle = COL.wallEdge;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + cell, y + cell);
  ctx.stroke();
  ctx.fillStyle = COL.clue;
  ctx.font = 'bold ' + Math.round(cell * 0.36) + 'px monospace';
  if (clue.right != null) {
    ctx.textAlign = 'right'; ctx.textBaseline = 'top';
    ctx.fillText(String(clue.right), x + cell - 3, y + 3);
  }
  if (clue.down != null) {
    ctx.textAlign = 'left'; ctx.textBaseline = 'bottom';
    ctx.fillText(String(clue.down), x + 3, y + cell - 2);
  }
}

function drawWhite(ctx, x, y, cell, r, c, pz, st) {
  const key = r + ',' + c;
  const bad = st.badCells.has(key);
  const sel = st.selected === key;
  ctx.fillStyle = bad ? COL.whiteBad : sel ? COL.whiteSel : COL.white;
  ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
  if (sel) {
    ctx.strokeStyle = COL.accent;
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 2, y + 2, cell - 4, cell - 4);
  }
  const v = st.fills[key];
  if (v) {
    ctx.fillStyle = COL.digit;
    ctx.font = 'bold ' + Math.round(cell * 0.56) + 'px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(v), x + cell / 2, y + cell / 2 + 1);
  }
}
