// Pixel Match Quest - gem and board rendering

const GEM_COLORS = [
  { main: '#ff5a6e', hi: '#ff9aa8', dark: '#a82430' }, // 0 red
  { main: '#4d9eff', hi: '#9cc8ff', dark: '#234f96' }, // 1 blue
  { main: '#5fd06a', hi: '#a6e8ac', dark: '#2c7a34' }, // 2 green
  { main: '#ffd23f', hi: '#ffe98c', dark: '#a8800f' }, // 3 yellow
  { main: '#b06ff0', hi: '#d8b0fa', dark: '#6a3aa0' }, // 4 purple
  { main: '#ff9436', hi: '#ffc285', dark: '#a85a14' }, // 5 orange
];

function pxr(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(Math.round(x), Math.round(y), Math.ceil(w), Math.ceil(h)); }

// draw a gem shape (distinct per colour) centred in a `s`-sized box at (x,y top-left)
function drawGemShape(ctx, color, x, y, s, scale) {
  const c = GEM_COLORS[color];
  const cx = x + s / 2, cy = y + s / 2;
  const r = s * 0.34 * (scale || 1);
  ctx.save();
  ctx.translate(cx, cy);
  ctx.lineWidth = Math.max(2, s * 0.06);
  ctx.strokeStyle = c.dark;
  ctx.fillStyle = c.main;
  ctx.beginPath();
  switch (color) {
    case 0: // round
      ctx.arc(0, 0, r, 0, 6.28); break;
    case 1: // diamond
      ctx.moveTo(0, -r * 1.2); ctx.lineTo(r, 0); ctx.lineTo(0, r * 1.2); ctx.lineTo(-r, 0); ctx.closePath(); break;
    case 2: // rounded square
      roundRect(ctx, -r * 0.92, -r * 0.92, r * 1.84, r * 1.84, r * 0.3); break;
    case 3: { // triangle
      ctx.moveTo(0, -r * 1.15); ctx.lineTo(r * 1.05, r * 0.8); ctx.lineTo(-r * 1.05, r * 0.8); ctx.closePath(); break;
    }
    case 4: { // hexagon
      for (let i = 0; i < 6; i++) {
        const a = Math.PI / 3 * i - Math.PI / 6;
        ctx[i ? 'lineTo' : 'moveTo'](Math.cos(a) * r * 1.08, Math.sin(a) * r * 1.08);
      }
      ctx.closePath(); break;
    }
    case 5: { // star
      for (let i = 0; i < 10; i++) {
        const a = Math.PI / 5 * i - Math.PI / 2;
        const rr = i % 2 ? r * 0.5 : r * 1.2;
        ctx[i ? 'lineTo' : 'moveTo'](Math.cos(a) * rr, Math.sin(a) * rr);
      }
      ctx.closePath(); break;
    }
  }
  ctx.fill();
  ctx.stroke();
  // highlight
  ctx.fillStyle = c.hi;
  ctx.beginPath();
  ctx.arc(-r * 0.32, -r * 0.36, r * 0.3, 0, 6.28);
  ctx.fill();
  ctx.restore();
}
function roundRect(ctx, x, y, w, h, r) {
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
}

// full cell render: gem + special markers + ice overlay
function drawCell(ctx, cell, x, y, s, t, scale) {
  if (!cell || cell.type === 'empty') return;
  if (cell.type === 'crate') {
    drawCrate(ctx, x, y, s, cell.layers);
    return;
  }
  if (cell.type === 'ingredient') {
    drawFruit(ctx, x, y, s, scale);
    if (cell.ice) drawIce(ctx, x, y, s, cell.ice);
    return;
  }
  // gem
  drawGemShape(ctx, cell.color, x, y, s, scale);
  if (cell.special) drawSpecialMark(ctx, cell.special, cell.color, x, y, s, t);
  if (cell.ice) drawIce(ctx, x, y, s, cell.ice);
}

function drawSpecialMark(ctx, special, color, x, y, s, t) {
  const cx = x + s / 2, cy = y + s / 2;
  ctx.save();
  if (special === 'rowH') {
    ctx.fillStyle = '#fff';
    pxr(ctx, x + s * 0.12, cy - 3, s * 0.76, 3, '#fff');
    pxr(ctx, x + s * 0.12, cy + 3, s * 0.76, 3, '#fff');
  } else if (special === 'rowV') {
    pxr(ctx, cx - 3, y + s * 0.12, 3, s * 0.76, '#fff');
    pxr(ctx, cx + 3, y + s * 0.12, 3, s * 0.76, '#fff');
  } else if (special === 'bomb') {
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.beginPath(); ctx.arc(cx, cy, s * 0.13, 0, 6.28); ctx.fill();
    ctx.strokeStyle = '#fff'; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(cx, cy, s * 0.24, 0, 6.28); ctx.stroke();
  } else if (special === 'color') {
    for (let i = 0; i < 6; i++) {
      ctx.fillStyle = GEM_COLORS[i].main;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.arc(cx, cy, s * 0.3, i / 6 * 6.28 + t, (i + 1) / 6 * 6.28 + t);
      ctx.closePath(); ctx.fill();
    }
    ctx.fillStyle = '#fff';
    ctx.beginPath(); ctx.arc(cx, cy, s * 0.1, 0, 6.28); ctx.fill();
  }
  ctx.restore();
}

function drawIce(ctx, x, y, s, layers) {
  ctx.save();
  ctx.fillStyle = layers >= 2 ? 'rgba(180,225,255,0.78)' : 'rgba(190,232,255,0.5)';
  ctx.fillRect(x + 2, y + 2, s - 4, s - 4);
  ctx.strokeStyle = '#eafaff';
  ctx.lineWidth = 2;
  ctx.strokeRect(x + 4, y + 4, s - 8, s - 8);
  // cracks
  ctx.beginPath();
  ctx.moveTo(x + s * 0.3, y + 4); ctx.lineTo(x + s * 0.45, y + s * 0.5); ctx.lineTo(x + s * 0.3, y + s - 4);
  ctx.moveTo(x + s * 0.55, y + 4); ctx.lineTo(x + s * 0.7, y + s * 0.55);
  ctx.stroke();
  ctx.restore();
}

function drawCrate(ctx, x, y, s, layers) {
  const m = 3;
  ctx.fillStyle = layers >= 2 ? '#6a4a28' : '#8a6238';
  ctx.fillRect(x + m, y + m, s - m * 2, s - m * 2);
  ctx.fillStyle = '#a8804a';
  ctx.fillRect(x + m, y + m, s - m * 2, 6);
  ctx.strokeStyle = '#4a3318';
  ctx.lineWidth = 3;
  ctx.strokeRect(x + m, y + m, s - m * 2, s - m * 2);
  ctx.beginPath();
  ctx.moveTo(x + m, y + m); ctx.lineTo(x + s - m, y + s - m);
  ctx.moveTo(x + s - m, y + m); ctx.lineTo(x + m, y + s - m);
  ctx.stroke();
}

function drawFruit(ctx, x, y, s, scale) {
  const cx = x + s / 2, cy = y + s / 2;
  const r = s * 0.26 * (scale || 1);
  ctx.fillStyle = '#3a8a2a';
  ctx.fillRect(cx - 2, cy - r - 8, 4, 10);
  ctx.fillStyle = '#5fc04a';
  ctx.beginPath(); ctx.ellipse(cx + 7, cy - r - 6, 7, 4, 0.6, 0, 6.28); ctx.fill();
  ctx.fillStyle = '#e8324a';
  ctx.beginPath(); ctx.arc(cx - r * 0.5, cy + 2, r, 0, 6.28); ctx.fill();
  ctx.beginPath(); ctx.arc(cx + r * 0.5, cy + 2, r, 0, 6.28); ctx.fill();
  ctx.fillStyle = '#ff9aa8';
  ctx.beginPath(); ctx.arc(cx - r * 0.8, cy - r * 0.3, r * 0.3, 0, 6.28); ctx.fill();
}
