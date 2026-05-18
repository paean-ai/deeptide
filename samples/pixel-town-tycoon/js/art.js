// Pixel Town Tycoon - pixel art for the town grid

function shade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function pr(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, Math.ceil(w), Math.ceil(h)); }

// grass tile background
function drawTile(ctx, gx, gy, hover) {
  const x = gx * TILE, y = gy * TILE;
  const checker = (gx + gy) & 1;
  ctx.fillStyle = checker ? '#4a8a3e' : '#458339';
  ctx.fillRect(x, y, TILE, TILE);
  ctx.fillStyle = checker ? '#54964a' : '#4f9044';
  const seed = (gx * 41 + gy * 17) % 9;
  if (seed < 3) ctx.fillRect(x + 8 + seed * 10, y + 12 + seed * 8, 5, 5);
  if (seed === 4) ctx.fillRect(x + 34, y + 30, 4, 4);
  // grid line
  ctx.strokeStyle = 'rgba(0,0,0,0.10)';
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, TILE - 1, TILE - 1);
}

function drawBlocked(ctx, gx, gy) {
  const cx = gx * TILE + TILE / 2, cy = gy * TILE + TILE / 2;
  ctx.fillStyle = 'rgba(0,0,0,0.22)';
  ctx.beginPath(); ctx.ellipse(cx, cy + 12, 16, 6, 0, 0, 6.28); ctx.fill();
  // a leafy tree
  ctx.fillStyle = '#5a3a22';
  ctx.fillRect(cx - 4, cy - 2, 8, 16);
  ctx.fillStyle = '#2f6b2c';
  ctx.beginPath(); ctx.arc(cx, cy - 8, 16, 0, 6.28); ctx.fill();
  ctx.fillStyle = '#3f8a3a';
  ctx.beginPath(); ctx.arc(cx - 5, cy - 12, 8, 0, 6.28); ctx.fill();
}

// draw a placed building centred in its tile
function drawBuilding(ctx, type, gx, gy, level, active, t) {
  const def = BUILDINGS[type];
  const cx = gx * TILE + TILE / 2;
  const baseY = gy * TILE + TILE - 8;
  const col = def.color;
  ctx.save();
  ctx.translate(cx, baseY);

  // shadow
  ctx.fillStyle = 'rgba(0,0,0,0.26)';
  ctx.beginPath(); ctx.ellipse(0, 2, 22, 7, 0, 0, 6.28); ctx.fill();

  if (!active) ctx.globalAlpha = 0.62;

  const wall = shade(col, 36), wallD = shade(col, -20), roof = shade(col, -8);
  // walls
  pr(ctx, -18, -28, 36, 30, wall);
  pr(ctx, -18, -28, 36, 5, shade(col, 60));
  pr(ctx, 13, -28, 5, 30, wallD);
  // roof
  ctx.fillStyle = roof;
  ctx.beginPath();
  ctx.moveTo(-24, -28); ctx.lineTo(0, -44); ctx.lineTo(24, -28); ctx.closePath();
  ctx.fill();
  ctx.fillStyle = shade(col, -28);
  ctx.beginPath();
  ctx.moveTo(0, -44); ctx.lineTo(24, -28); ctx.lineTo(18, -28); ctx.lineTo(0, -40); ctx.closePath();
  ctx.fill();
  // door
  pr(ctx, -6, -16, 12, 18, shade(col, -45));
  pr(ctx, -4, -14, 8, 14, shade(col, -28));

  // per-type feature
  drawFeature(ctx, type, col, active, t);

  // level pips
  for (let i = 0; i < level; i++) {
    ctx.fillStyle = '#ffd34d';
    ctx.fillRect(-15 + i * 7, -52, 5, 5);
    ctx.fillStyle = '#a8780f';
    ctx.fillRect(-15 + i * 7, -52, 5, 1);
  }
  ctx.globalAlpha = 1;
  ctx.restore();
}

function drawFeature(ctx, type, col, active, t) {
  switch (type) {
    case 'cottage':
      pr(ctx, -14, -10, 7, 7, '#ffe9a8'); // window
      pr(ctx, 8, -42, 6, 10, shade(col, -50)); // chimney
      break;
    case 'farm':
      for (let i = 0; i < 3; i++) pr(ctx, -22 + i * 16, 0, 10, 4, '#e8c44a');
      break;
    case 'well':
      pr(ctx, -10, -38, 20, 5, '#6a4a2a');
      pr(ctx, -3, -32, 6, 8, '#5fb8e0');
      break;
    case 'lumber':
      pr(ctx, -24, -6, 16, 8, '#a8753a');
      pr(ctx, -24, -6, 16, 3, '#caa14a');
      break;
    case 'mine':
      pr(ctx, -10, -14, 20, 14, '#2a2535');
      pr(ctx, -5, -10, 10, 10, '#5a4a3a');
      break;
    case 'mill': {
      const a = active ? t * 2 : 0;
      ctx.save(); ctx.translate(0, -36); ctx.rotate(a);
      for (let i = 0; i < 4; i++) {
        ctx.rotate(Math.PI / 2);
        pr(ctx, 0, -3, 18, 6, '#e8e0c8');
      }
      ctx.restore();
      break;
    }
    case 'sawmill': {
      const a = active ? t * 5 : 0;
      ctx.save(); ctx.translate(-16, -8); ctx.rotate(a);
      ctx.fillStyle = '#cfd6e2';
      for (let i = 0; i < 8; i++) { ctx.rotate(0.785); pr(ctx, 0, -2, 9, 4, '#cfd6e2'); }
      ctx.restore();
      break;
    }
    case 'bakery':
      pr(ctx, 9, -44, 6, 12, shade(col, -50));
      if (active) {
        ctx.fillStyle = 'rgba(220,220,220,' + (0.5 + Math.sin(t * 3) * 0.2) + ')';
        ctx.beginPath(); ctx.arc(12, -48 - (t * 6 % 12), 4, 0, 6.28); ctx.fill();
      }
      break;
    case 'smithy':
      pr(ctx, -22, -8, 12, 8, '#3a3540');
      if (active) pr(ctx, -19, -12, 6, 5, '#ff7a3a');
      break;
    case 'market':
      for (let i = 0; i < 5; i++)
        pr(ctx, -22 + i * 9, -32, 9, 8, i % 2 ? '#e85d5d' : '#f4f0e6');
      break;
    case 'warehouse':
      pr(ctx, -14, -22, 28, 6, shade(col, 50));
      pr(ctx, -2, -22, 4, 24, shade(col, -45));
      break;
  }
}

// build-palette / panel icon
function drawBuildingIcon(ctx, type, cx, cy, scale) {
  ctx.save();
  ctx.translate(cx, cy + 14 * scale);
  ctx.scale(scale, scale);
  const def = BUILDINGS[type], col = def.color;
  pr(ctx, -16, -26, 32, 28, shade(col, 36));
  ctx.fillStyle = shade(col, -8);
  ctx.beginPath(); ctx.moveTo(-21, -26); ctx.lineTo(0, -40); ctx.lineTo(21, -26); ctx.closePath(); ctx.fill();
  pr(ctx, -5, -14, 10, 16, shade(col, -45));
  drawFeature(ctx, type, col, true, 0);
  ctx.restore();
}

// little floating coin/pop number
function drawFloatText(ctx, x, y, str, color, alpha) {
  ctx.globalAlpha = alpha;
  ctx.font = 'bold 14px monospace';
  ctx.textAlign = 'center';
  ctx.fillStyle = '#000';
  ctx.fillText(str, x + 1, y + 1);
  ctx.fillStyle = color;
  ctx.fillText(str, x, y);
  ctx.globalAlpha = 1;
}
