// Pixel Slide - backdrop and tile rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#222a44');
  g.addColorStop(1, '#080a12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A tile's colour comes from its HOME position, so a solved board forms a
// smooth gradient and a misplaced tile shows the "wrong" hue in its spot.
function tileColor(value, n) {
  const h = value - 1;
  const hr = (h / n) | 0, hc = h % n;
  const hue = Math.round(8 + (hc / Math.max(1, n - 1)) * 286);
  const light = Math.round(40 + (hr / Math.max(1, n - 1)) * 24);
  return 'hsl(' + hue + ', 62%, ' + light + '%)';
}

function drawTile(ctx, x, y, size, value, n) {
  const s = size - 3;
  ctx.fillStyle = tileColor(value, n);
  ctx.fillRect(x + 1.5, y + 1.5, s, s);
  ctx.fillStyle = 'rgba(255,255,255,0.26)';
  ctx.fillRect(x + 1.5, y + 1.5, s, 4);
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.fillRect(x + 1.5, y + size - 5.5, s, 4);
  ctx.fillStyle = '#0c0e16';
  ctx.font = '900 ' + Math.round(size * 0.4) + 'px ui-monospace, monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(value), x + size / 2, y + size / 2 + 1);
}

function drawBoardFrame(ctx, gx, gy, n, cell) {
  ctx.fillStyle = '#10131f';
  ctx.fillRect(gx - 3, gy - 3, n * cell + 6, n * cell + 6);
}
