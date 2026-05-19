// Pixel Sky Climber - pixel art for the climber, platforms, pickups, monsters.

function scShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function SR(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, Math.ceil(w), Math.ceil(h)); }

// The climber. cx,cy = centre; `squash` -1..1 (negative = squashed on landing,
// positive = stretched while rising); facing 1/-1; jet = jetpack active.
function drawClimber(ctx, cx, cy, squash, facing, jet, t) {
  const sx = 1 - squash * 0.3, sy = 1 + squash * 0.3;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(facing * sx, sy);
  const R = (x, y, w, h, c) => SR(ctx, x, y, w, h, c);
  // shadow puff
  if (jet) {
    R(-6, 14, 12, 6 + Math.abs(Math.sin(t * 30)) * 8, '#ffd24d');
    R(-4, 18, 8, 6 + Math.abs(Math.cos(t * 30)) * 7, '#ff8a3c');
  }
  // springy legs
  R(-7, 8, 4, 8, '#3a4658');
  R(3, 8, 4, 8, '#3a4658');
  R(-8, 15, 6, 3, '#222a37');
  R(2, 15, 6, 3, '#222a37');
  // body
  R(-10, -10, 20, 19, '#5fd9c0');
  R(-10, -10, 20, 5, '#9af0e0');
  R(-10, 5, 20, 4, '#3a9d8c');
  // belly patch
  R(-5, -2, 10, 8, '#e8fbf5');
  // head/face area - big eyes
  R(-8, -8, 6, 7, '#10131c');
  R(2, -8, 6, 7, '#10131c');
  R(-6, -7, 3, 3, '#ffffff');
  R(4, -7, 3, 3, '#ffffff');
  // antenna
  R(-1, -15, 2, 5, '#3a9d8c');
  R(-3, -18, 6, 4, '#f4c85a');
  if (jet) { // jetpack on the back
    R(8, -8, 5, 14, '#9aa6b8');
    R(8, -8, 5, 3, '#c9d2e0');
  }
  ctx.restore();
}

function drawPlatform(ctx, p) {
  const x = p.x, y = p.y, w = p.w, h = PLAT_H;
  if (p.type === 'breakable' && p.broken) {
    // crumbling - two falling halves
    SR(ctx, x, y, w / 2 - 3, h, '#7a5230');
    SR(ctx, x + w / 2 + 3, y, w / 2 - 3, h, '#7a5230');
    return;
  }
  const def = PLAT_TYPES[p.type];
  const base = def.color, dk = scShade(base, -54), lt = scShade(base, 42);
  SR(ctx, x, y, w, h, dk);
  SR(ctx, x, y, w, h - 4, base);
  SR(ctx, x, y, w, 3, lt);
  if (p.type === 'normal') {
    for (let i = 4; i < w - 4; i += 12) SR(ctx, x + i, y - 3, 5, 3, lt);  // grass tufts
  } else if (p.type === 'moving') {
    SR(ctx, x + w / 2 - 8, y + 4, 16, 4, lt);                            // tech stripe
    SR(ctx, x + 4, y + 4, 4, 4, '#10131c');
    SR(ctx, x + w - 8, y + 4, 4, 4, '#10131c');
  } else if (p.type === 'breakable') {
    SR(ctx, x + w * 0.34, y, 3, h, dk);                                  // cracks
    SR(ctx, x + w * 0.64, y + 3, 3, h - 3, dk);
  } else if (p.type === 'spring') {
    const sx = x + w / 2;
    SR(ctx, sx - 7, y - 8, 14, 3, '#c9d2e0');                            // coil
    SR(ctx, sx - 5, y - 5, 10, 2, '#9aa6b8');
    SR(ctx, sx - 7, y - 11, 14, 3, '#e8eef6');
  }
}

function drawCoin(ctx, x, y, t) {
  const sw = Math.abs(Math.sin(t * 5)) * 8 + 3;
  SR(ctx, x - sw / 2, y - 8, sw, 16, '#f4c85a');
  SR(ctx, x - sw / 2, y - 8, sw, 4, '#ffe9a0');
  SR(ctx, x - 1, y - 4, 2, 8, '#a87a1e');
}

function drawMonster(ctx, x, y, t) {
  const bob = Math.sin(t * 3 + x) * 3;
  const cy = y + bob;
  SR(ctx, x - 13, cy - 9, 26, 18, '#b85fd0');
  SR(ctx, x - 13, cy - 9, 26, 5, '#d68aea');
  SR(ctx, x - 13, cy + 5, 26, 4, '#7a3a90');
  // spikes
  for (let i = -10; i <= 10; i += 8) SR(ctx, x + i, cy - 13, 5, 5, '#7a3a90');
  // eyes
  SR(ctx, x - 8, cy - 4, 6, 6, '#ffffff');
  SR(ctx, x + 2, cy - 4, 6, 6, '#ffffff');
  SR(ctx, x - 6, cy - 2, 3, 3, '#10131c');
  SR(ctx, x + 4, cy - 2, 3, 3, '#10131c');
  // fang
  SR(ctx, x - 3, cy + 4, 6, 4, '#ffffff');
}

function drawJetpack(ctx, x, y, t) {
  const bob = Math.sin(t * 4) * 2;
  SR(ctx, x - 8, y - 10 + bob, 16, 20, '#9aa6b8');
  SR(ctx, x - 8, y - 10 + bob, 16, 4, '#d9e0ea');
  SR(ctx, x - 5, y - 6 + bob, 10, 8, '#5fd9ff');
  SR(ctx, x - 6, y + 10 + bob, 4, 5, '#ff8a3c');
  SR(ctx, x + 2, y + 10 + bob, 4, 5, '#ff8a3c');
}

// Parallax sky backdrop. `scroll` is the accumulated climb in px.
function drawSky(ctx, scroll, t) {
  const band = Math.min(1, scroll / 60000);
  const top = scShade('#1d3a6b', Math.round(band * 60 - 30));
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, top);
  g.addColorStop(1, scShade('#3f6ea8', Math.round(-band * 50)));
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // stars appear high up
  if (band > 0.3) {
    ctx.globalAlpha = (band - 0.3) * 1.4;
    for (let i = 0; i < 28; i++) {
      const sx = (i * 53) % VW, sy = (i * 89 + scroll * 0.05) % VH;
      ctx.fillStyle = '#eef4ff';
      ctx.fillRect(sx, sy, 2, 2);
    }
    ctx.globalAlpha = 1;
  }
  // parallax clouds
  ctx.fillStyle = 'rgba(255,255,255,0.16)';
  for (let i = 0; i < 6; i++) {
    const cy = (i * 150 + scroll * 0.12) % (VH + 80) - 40;
    const cx = (i * 137 + 30) % VW;
    ctx.fillRect(cx, cy, 60, 16);
    ctx.fillRect(cx + 14, cy - 10, 34, 12);
  }
}
