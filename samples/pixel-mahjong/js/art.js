// Pixel Mahjong - backdrop, tile bodies and tile-face symbols.

const SYMBOL_COLORS = [
  '#e8554f', '#4a9be8', '#5fc06e', '#f2cf3f', '#9a6cd8',
  '#ef9b3e', '#4fd6d6', '#ff7db0', '#a8d84a',
];

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1b3326');
  g.addColorStop(1, '#08120d');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// A tile-face symbol centred at (cx, cy). Distinct shape AND colour per type.
function drawSymbol(ctx, type, cx, cy, s) {
  ctx.fillStyle = SYMBOL_COLORS[type];
  const h = s / 2;
  if (type === 0) {                                  // disc
    ctx.beginPath(); ctx.arc(cx, cy, h, 0, Math.PI * 2); ctx.fill();
  } else if (type === 1) {                           // square
    ctx.fillRect(cx - h, cy - h, s, s);
  } else if (type === 2) {                           // triangle
    ctx.beginPath();
    ctx.moveTo(cx, cy - h); ctx.lineTo(cx + h, cy + h); ctx.lineTo(cx - h, cy + h);
    ctx.closePath(); ctx.fill();
  } else if (type === 3) {                           // diamond
    ctx.beginPath();
    ctx.moveTo(cx, cy - h); ctx.lineTo(cx + h, cy); ctx.lineTo(cx, cy + h); ctx.lineTo(cx - h, cy);
    ctx.closePath(); ctx.fill();
  } else if (type === 4) {                           // ring
    ctx.beginPath(); ctx.arc(cx, cy, h, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#f2eddc';
    ctx.beginPath(); ctx.arc(cx, cy, h * 0.46, 0, Math.PI * 2); ctx.fill();
  } else if (type === 5) {                           // star
    ctx.beginPath();
    for (let i = 0; i < 10; i++) {
      const a = -Math.PI / 2 + i * Math.PI / 5;
      const rr = i % 2 ? h * 0.45 : h;
      const fn = i ? 'lineTo' : 'moveTo';
      ctx[fn](cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
    }
    ctx.closePath(); ctx.fill();
  } else if (type === 6) {                           // plus
    const t = s * 0.3;
    ctx.fillRect(cx - t / 2, cy - h, t, s);
    ctx.fillRect(cx - h, cy - t / 2, s, t);
  } else if (type === 7) {                           // heart
    ctx.beginPath();
    ctx.arc(cx - h * 0.45, cy - h * 0.2, h * 0.5, 0, Math.PI * 2);
    ctx.arc(cx + h * 0.45, cy - h * 0.2, h * 0.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.beginPath();
    ctx.moveTo(cx - h, cy - h * 0.1); ctx.lineTo(cx + h, cy - h * 0.1); ctx.lineTo(cx, cy + h);
    ctx.closePath(); ctx.fill();
  } else {                                           // hexagon
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      const a = Math.PI / 6 + i * Math.PI / 3;
      const fn = i ? 'lineTo' : 'moveTo';
      ctx[fn](cx + Math.cos(a) * h, cy + Math.sin(a) * h);
    }
    ctx.closePath(); ctx.fill();
  }
}

// A mahjong tile at (x, y, w, h). state: 'free' | 'blocked' | 'selected' | 'hint'.
function drawTile(ctx, x, y, w, h, type, state) {
  const d = 5;
  // stacked edge (depth)
  ctx.fillStyle = '#8a7a4e';
  ctx.fillRect(x + d, y + d, w, h);
  // face
  ctx.fillStyle = state === 'selected' ? '#fff4c2'
    : state === 'hint' ? '#d6f0c8' : '#f2eddc';
  ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = state === 'selected' ? '#f2cf3f'
    : state === 'hint' ? '#5fc06e' : '#b3ad94';
  ctx.lineWidth = state === 'selected' || state === 'hint' ? 2.5 : 1;
  ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
  ctx.fillStyle = 'rgba(255,255,255,0.5)';
  ctx.fillRect(x + 2, y + 2, w - 4, 2);

  drawSymbol(ctx, type, x + w / 2, y + h / 2, Math.min(w, h) * 0.46);

  if (state === 'blocked') {
    ctx.fillStyle = 'rgba(20, 28, 22, 0.42)';
    ctx.fillRect(x, y, w, h);
  }
}
