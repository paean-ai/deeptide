// Pixel Glyphs - rendering.

const COL = {
  litA: '#7be0c8', litB: '#ffd86b', dark: '#1d2236', darkEdge: '#2e3552',
  frame: '#3a4368', rune: '#0c1020', runeLit: '#10342c',
};

// a small 5x5 rune bitmap stamped on each glyph (purely decorative)
const RUNE = [
  '01110',
  '10001',
  '01010',
  '10001',
  '01110',
];

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1a2138');
  g.addColorStop(1, '#0a0d18');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(n) {
  const cell = Math.min(74, Math.floor(304 / n));
  const span = cell * n;
  return { cell, gx: Math.round((VW - span) / 2), gy: Math.round(126 + (300 - span) / 2) };
}

function drawBoard(ctx, n, grid, geom, pulse) {
  const { cell, gx, gy } = geom;
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      const lit = grid[r * n + c];
      const x = gx + c * cell, y = gy + r * cell;
      drawGlyph(ctx, x, y, cell, lit, (r + c) % 2, pulse);
    }
  }
}

function drawGlyph(ctx, x, y, cell, lit, alt, pulse) {
  const m = 3, sz = cell - m * 2;
  if (lit) {
    const glow = 0.5 + 0.5 * Math.sin(pulse);
    ctx.fillStyle = `rgba(123,224,200,${0.12 + glow * 0.1})`;
    ctx.fillRect(x, y, cell, cell);
    ctx.fillStyle = alt ? COL.litA : COL.litB;
  } else {
    ctx.fillStyle = COL.dark;
  }
  ctx.fillRect(x + m, y + m, sz, sz);
  ctx.strokeStyle = lit ? '#ffffff55' : COL.darkEdge;
  ctx.lineWidth = 1;
  ctx.strokeRect(x + m + 0.5, y + m + 0.5, sz - 1, sz - 1);
  // rune stamp
  const px = sz / 7;
  ctx.fillStyle = lit ? COL.runeLit : '#2a3050';
  for (let rr = 0; rr < 5; rr++) {
    for (let cc = 0; cc < 5; cc++) {
      if (RUNE[rr][cc] === '1') {
        ctx.fillRect(x + m + px + cc * px, y + m + px + rr * px, px, px);
      }
    }
  }
}
