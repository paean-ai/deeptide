// Pixel Color Cascade - board, tile and swatch rendering.

function shade(hex, f) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const adj = v => Math.max(0, Math.min(255,
    Math.round(f < 0 ? v * (1 + f) : v + (255 - v) * f)));
  return `rgb(${adj(r)},${adj(g)},${adj(b)})`;
}

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1b2436');
  g.addColorStop(1, '#10151f');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// Bevelled pixel tile of side s at (x,y) using palette index colorIdx.
function drawTile(ctx, x, y, s, colorIdx) {
  const col = PALETTE[colorIdx];
  ctx.fillStyle = shade(col, -0.5);
  ctx.fillRect(x, y, s, s);
  ctx.fillStyle = col;
  ctx.fillRect(x + 1, y + 1, s - 2, s - 2);
  ctx.fillStyle = shade(col, 0.36);
  ctx.fillRect(x + 1, y + 1, s - 2, 2);
  ctx.fillRect(x + 1, y + 1, 2, s - 2);
  ctx.fillStyle = shade(col, -0.3);
  ctx.fillRect(x + 1, y + s - 3, s - 2, 2);
  ctx.fillRect(x + s - 3, y + 1, 2, s - 2);
}

function drawGrid(ctx, L, cells, n, clock) {
  ctx.fillStyle = '#080b11';
  ctx.fillRect(L.bx - 5, L.by - 5, L.bs + 10, L.bs + 10);
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const c = cells[y * n + x];
      const shown = clock >= c.flipAt ? c.c : c.prev;
      const px = L.bx + x * L.cell, py = L.by + y * L.cell;
      drawTile(ctx, px, py, L.cell + 0.6, shown);
      const age = clock - c.flipAt;
      if (age >= 0 && age < 0.18) {
        ctx.globalAlpha = (0.18 - age) / 0.18 * 0.75;
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(px, py, L.cell + 0.6, L.cell + 0.6);
        ctx.globalAlpha = 1;
      }
    }
  }
  // origin marker (top-left tile)
  ctx.fillStyle = 'rgba(255,255,255,0.92)';
  ctx.fillRect(L.bx + L.cell * 0.5 - 2, L.by + L.cell * 0.5 - 2, 4, 4);
}

function drawSwatches(ctx, sw, curColor) {
  for (let i = 0; i < sw.length; i++) {
    const s = sw[i];
    ctx.fillStyle = '#080b11';
    ctx.fillRect(s.x - 3, s.y - 3, s.w + 6, s.w + 6);
    drawTile(ctx, s.x, s.y, s.w, i);
    if (i === curColor) {
      ctx.fillStyle = 'rgba(8,11,17,0.55)';
      ctx.fillRect(s.x, s.y, s.w, s.w);
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2;
      ctx.strokeRect(s.x + 2, s.y + 2, s.w - 4, s.w - 4);
    }
  }
}
