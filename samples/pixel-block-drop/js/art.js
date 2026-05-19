// Pixel Block Drop - board, piece, panel and button rendering.

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
  g.addColorStop(0, '#161d30');
  g.addColorStop(1, '#0a0e18');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawCell(ctx, x, y, s, color, alpha) {
  if (alpha != null) ctx.globalAlpha = alpha;
  ctx.fillStyle = shade(color, -0.5);
  ctx.fillRect(x, y, s, s);
  ctx.fillStyle = color;
  ctx.fillRect(x + 1, y + 1, s - 2, s - 2);
  ctx.fillStyle = shade(color, 0.42);
  ctx.fillRect(x + 1, y + 1, s - 2, 2);
  ctx.fillRect(x + 1, y + 1, 2, s - 2);
  ctx.fillStyle = shade(color, -0.32);
  ctx.fillRect(x + 1, y + s - 3, s - 2, 2);
  ctx.fillRect(x + s - 3, y + 1, 2, s - 2);
  ctx.globalAlpha = 1;
}

function drawBoard(ctx, board, flashRows, flashT) {
  ctx.fillStyle = '#070a12';
  ctx.fillRect(BX - 4, BY - 4, COLS * CELL + 8, ROWS * CELL + 8);
  for (let y = 0; y < ROWS; y++) {
    for (let x = 0; x < COLS; x++) {
      const px = BX + x * CELL, py = BY + y * CELL;
      if (board[y][x]) {
        drawCell(ctx, px, py, CELL, board[y][x]);
      } else {
        ctx.fillStyle = (x + y) % 2 ? '#141a2c' : '#111624';
        ctx.fillRect(px, py, CELL, CELL);
      }
    }
  }
  if (flashRows && flashRows.length) {
    ctx.globalAlpha = Math.max(0, 1 - flashT / 0.26);
    ctx.fillStyle = '#ffffff';
    for (const ry of flashRows) ctx.fillRect(BX, BY + ry * CELL, COLS * CELL, CELL);
    ctx.globalAlpha = 1;
  }
}

// draw a live piece (or its ghost) on the board
function drawPiece(ctx, id, rot, px, py, ghost) {
  const color = PIECES[id].color;
  for (const [ox, oy] of PIECES[id].states[rot]) {
    const cy = py + oy;
    if (cy < 0) continue;
    const x = BX + (px + ox) * CELL, y = BY + cy * CELL;
    if (ghost) {
      ctx.strokeStyle = color;
      ctx.globalAlpha = 0.55;
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 2, y + 2, CELL - 4, CELL - 4);
      ctx.globalAlpha = 1;
    } else {
      drawCell(ctx, x, y, CELL, color);
    }
  }
}

function drawMiniPiece(ctx, id, boxX, boxY, boxW, c) {
  const cells = PIECES[id].states[0];
  let minX = 9, maxX = 0, minY = 9, maxY = 0;
  for (const [ox, oy] of cells) {
    minX = Math.min(minX, ox); maxX = Math.max(maxX, ox);
    minY = Math.min(minY, oy); maxY = Math.max(maxY, oy);
  }
  const w = (maxX - minX + 1) * c, h = (maxY - minY + 1) * c;
  const sx = boxX + (boxW - w) / 2, sy = boxY + (boxW - h) / 2;
  for (const [ox, oy] of cells)
    drawCell(ctx, sx + (ox - minX) * c, sy + (oy - minY) * c, c, PIECES[id].color);
}

function drawButtons(ctx, buttons, pressed) {
  for (const b of buttons) {
    const down = pressed && pressed.has(b.id);
    ctx.fillStyle = '#070a12';
    ctx.fillRect(b.x - 2, b.y - 2, b.w + 4, b.h + 4);
    ctx.fillStyle = down ? '#3a4a6e' : '#26304a';
    ctx.fillRect(b.x, b.y, b.w, b.h);
    ctx.fillStyle = down ? '#5fc06e' : '#8fa2c8';
    ctx.font = '900 18px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(b.glyph, b.x + b.w / 2, b.y + b.h / 2 + 1);
  }
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
}
