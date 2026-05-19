// Pixel Turbo Racer - pixel art: cars, pickups, hazards, roadside props.

function trShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function TR(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w, h); }

// Top-down car centred on (cx, cy). Cars face up the screen.
function drawCar(ctx, cx, cy, color, isPlayer, flash) {
  const w = isPlayer ? PLAYER.w : 32, h = isPlayer ? PLAYER.h : 50;
  const x = cx - w / 2, y = cy - h / 2;
  const body = flash > 0 ? '#ffffff' : color;
  const dk = trShade(color, -54), lt = trShade(color, 44);
  // shadow
  ctx.fillStyle = 'rgba(0,0,0,0.32)';
  ctx.fillRect(x + 3, y + 6, w, h);
  // tyres
  TR(ctx, x - 3, y + 7, 6, 12, '#181820');
  TR(ctx, x + w - 3, y + 7, 6, 12, '#181820');
  TR(ctx, x - 3, y + h - 19, 6, 12, '#181820');
  TR(ctx, x + w - 3, y + h - 19, 6, 12, '#181820');
  // body
  TR(ctx, x, y, w, h, body);
  TR(ctx, x, y, 3, h, lt);
  TR(ctx, x + w - 3, y, 3, h, dk);
  TR(ctx, x + 4, y, w - 8, 3, lt);          // front lip
  TR(ctx, x + 4, y + h - 3, w - 8, 3, dk);  // rear lip
  // windshield + rear window
  TR(ctx, x + 5, y + 9, w - 10, 11, '#1c2a3a');
  TR(ctx, x + 6, y + 10, w - 12, 4, '#4f7da8');
  TR(ctx, x + 5, y + h - 19, w - 10, 9, '#1c2a3a');
  // roof
  TR(ctx, x + 6, y + 22, w - 12, h - 42, isPlayer ? trShade(color, 18) : color);
  if (isPlayer) {
    TR(ctx, cx - 2, y + 22, 4, h - 42, '#f4f7ff');  // racing stripe
  }
  // lights
  TR(ctx, x + 3, y + 1, 5, 3, '#fff2b0');
  TR(ctx, x + w - 8, y + 1, 5, 3, '#fff2b0');
  TR(ctx, x + 3, y + h - 4, 5, 3, '#ff5a5a');
  TR(ctx, x + w - 8, y + h - 4, 5, 3, '#ff5a5a');
}

function drawCoin(ctx, x, y, t) {
  const sw = Math.abs(Math.sin(t * 5)) * 8 + 3;
  TR(ctx, x - sw / 2, y - 9, sw, 18, '#ffd24d');
  TR(ctx, x - sw / 2, y - 9, sw, 4, '#ffe9a0');
  ctx.fillStyle = '#a87a1e';
  ctx.fillRect(x - 1, y - 4, 2, 8);
}

function drawNitro(ctx, x, y, t) {
  const pulse = Math.sin(t * 6) * 2;
  TR(ctx, x - 11, y - 13 - pulse, 22, 26, '#10283a');
  TR(ctx, x - 8, y - 10 - pulse, 16, 20, '#7ad0ff');
  TR(ctx, x - 8, y - 10 - pulse, 16, 5, '#d6f4ff');
  ctx.fillStyle = '#10283a';
  ctx.font = 'bold 13px ui-monospace, monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('N', x, y - pulse);
  ctx.textBaseline = 'alphabetic';
  ctx.textAlign = 'left';
}

function drawCone(ctx, x, y) {
  TR(ctx, x - 11, y + 8, 22, 5, '#c9622a');
  TR(ctx, x - 7, y - 2, 14, 10, '#ff8a3c');
  TR(ctx, x - 4, y - 12, 8, 10, '#ff8a3c');
  TR(ctx, x - 6, y + 1, 12, 3, '#fff2e0');
}

function drawOil(ctx, x, y, t) {
  ctx.fillStyle = '#0d0d14';
  ctx.beginPath();
  ctx.ellipse(x, y, 22, 12, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = 'rgba(120,150,200,0.35)';
  ctx.fillRect(x - 8, y - 4, 7, 4);
  ctx.fillRect(x + 3, y + 1, 5, 3);
}

// Roadside prop - a chunky pixel tree.
function drawTree(ctx, x, y) {
  TR(ctx, x - 3, y, 6, 14, '#5a3f24');
  TR(ctx, x - 13, y - 18, 26, 20, '#2f7d3a');
  TR(ctx, x - 9, y - 28, 18, 14, '#3c9a48');
  TR(ctx, x - 9, y - 28, 18, 4, '#5fc068');
}
