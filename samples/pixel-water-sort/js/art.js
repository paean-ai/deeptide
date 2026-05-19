// Pixel Water Sort - backdrop and tube rendering.

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1a2d3a');
  g.addColorStop(1, '#070c12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A glass tube at (x, y) of given width / unit height, holding `tube`
// (bottom-up colour indices). selected lifts it; pourHi flags a legal target.
function drawTube(ctx, x, y, w, unitH, tube, selected, pourHi) {
  const lift = selected ? -10 : 0;
  const ty = y + lift;
  const h = unitH * TUBE_CAP;
  // liquid segments, bottom-up
  for (let i = 0; i < tube.length; i++) {
    ctx.fillStyle = COLORS[tube[i]];
    const sy = ty + h - (i + 1) * unitH;
    ctx.fillRect(x + 2, sy, w - 4, unitH);
    ctx.fillStyle = 'rgba(255,255,255,0.16)';
    ctx.fillRect(x + 2, sy, w - 4, 3);
  }
  // glass walls
  ctx.strokeStyle = pourHi ? '#f2cf3f' : (selected ? '#4fc6e0' : '#5f7a88');
  ctx.lineWidth = pourHi || selected ? 2.5 : 1.5;
  ctx.beginPath();
  ctx.moveTo(x + 0.5, ty - 4);
  ctx.lineTo(x + 0.5, ty + h);
  ctx.lineTo(x + w - 0.5, ty + h);
  ctx.lineTo(x + w - 0.5, ty - 4);
  ctx.stroke();
  // glass sheen
  ctx.fillStyle = 'rgba(255,255,255,0.08)';
  ctx.fillRect(x + 3, ty - 4, 3, h + 4);
}
