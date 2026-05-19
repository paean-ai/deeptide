// Pixel Orb Fusion - board and orb rendering.

function drawBoard(ctx) {
  ctx.fillStyle = '#1a1726';
  ctx.fillRect(0, 0, BOARD, BOARD);
  for (let r = 0; r < GRID; r++) {
    for (let c = 0; c < GRID; c++) {
      const x = c * CELL, y = r * CELL;
      ctx.fillStyle = '#241f36';
      ctx.fillRect(x + 4, y + 4, CELL - 8, CELL - 8);
      ctx.fillStyle = '#2c2742';
      ctx.fillRect(x + 4, y + 4, CELL - 8, 4);
    }
  }
}

// Draw an orb at pixel (px, py) = top-left of its cell. `scale` for pop anim.
function drawOrb(ctx, px, py, tier, scale) {
  const s = CELL * scale;
  const cx = px + CELL / 2, cy = py + CELL / 2;
  const x = cx - s / 2, y = cy - s / 2;
  const hue = orbHue(tier);
  const m = 9 * scale;                       // inner margin
  // body
  ctx.fillStyle = `hsl(${hue}, 16%, 12%)`;
  ctx.fillRect(x + m - 3, y + m - 3, s - (m - 3) * 2, s - (m - 3) * 2);
  ctx.fillStyle = `hsl(${hue}, 64%, 52%)`;
  ctx.fillRect(x + m, y + m, s - m * 2, s - m * 2);
  ctx.fillStyle = `hsl(${hue}, 72%, 66%)`;
  ctx.fillRect(x + m, y + m, s - m * 2, Math.max(3, 6 * scale));
  ctx.fillStyle = `hsl(${hue}, 50%, 38%)`;
  ctx.fillRect(x + m, y + s - m - Math.max(3, 6 * scale), s - m * 2, Math.max(3, 6 * scale));
  // core glint
  ctx.fillStyle = `hsl(${hue}, 80%, 82%)`;
  ctx.fillRect(x + m + 6 * scale, y + m + 6 * scale, 7 * scale, 7 * scale);
  // value text
  const val = orbValue(tier);
  const fs = Math.round((val >= 1024 ? 24 : val >= 128 ? 28 : 34) * scale);
  ctx.fillStyle = tier <= 2 ? '#2a2438' : '#fff';
  ctx.font = `900 ${fs}px ui-monospace, Menlo, monospace`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(val), cx, cy + 1);
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
}
