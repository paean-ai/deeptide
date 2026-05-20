// Pixel-art rendering for Pixel Aquarium. 360x480 world units.

const PALETTE = {
  bg:        '#091128',
  glass:     '#1c2245',
  glassEdge: '#3a4274',
  air:       '#cfe3ff',
  airShade:  '#9aa6cc',
  water:     '#4a9be8',
  waterDeep: '#1f5494',
  waterCrest:'#cce6ff',
  bubble:    '#dde6ff',
  border:    '#07091a',
  hint:      '#f7e69a',
  conflict:  '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function gridGeometry(n) {
  const size = 300;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 64;
  return { cell, total, ox, oy };
}

function drawScene(ctx, p, marks, conflicts) {
  const { cell, total, ox, oy } = gridGeometry(p.n);
  const owner = new Array(p.n * p.n);
  p.regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  // Column counts.
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold ' + ((cell * 0.5) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let x = 0; x < p.n; x++) {
    ctx.fillText(String(p.cc[x]), ox + x * cell + cell / 2, oy - 12);
  }
  for (let y = 0; y < p.n; y++) {
    ctx.fillText(String(p.rc[y]), ox - 14, oy + y * cell + cell / 2);
  }
  // Cells.
  for (let y = 0; y < p.n; y++) for (let x = 0; x < p.n; x++) {
    const i = y * p.n + x;
    const px = ox + x * cell, py = oy + y * cell;
    drawCell(ctx, marks[i], px, py, cell, conflicts && conflicts.has(i));
  }
  // Thin grid lines.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= p.n; i++) {
    ctx.beginPath(); ctx.moveTo(ox + i * cell + 0.5, oy); ctx.lineTo(ox + i * cell + 0.5, oy + total); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(ox, oy + i * cell + 0.5); ctx.lineTo(ox + total, oy + i * cell + 0.5); ctx.stroke();
  }
  // Thick tank borders.
  ctx.strokeStyle = PALETTE.glassEdge;
  ctx.lineWidth = 2;
  for (let y = 0; y < p.n; y++) for (let x = 0; x < p.n; x++) {
    const i = y * p.n + x;
    const px = ox + x * cell, py = oy + y * cell;
    const reg = owner[i];
    if (x === p.n - 1 || owner[y * p.n + x + 1] !== reg) {
      ctx.beginPath(); ctx.moveTo(px + cell + 0.5, py); ctx.lineTo(px + cell + 0.5, py + cell); ctx.stroke();
    }
    if (y === p.n - 1 || owner[(y + 1) * p.n + x] !== reg) {
      ctx.beginPath(); ctx.moveTo(px, py + cell + 0.5); ctx.lineTo(px + cell, py + cell + 0.5); ctx.stroke();
    }
    if (y === 0) { ctx.beginPath(); ctx.moveTo(px, py + 0.5); ctx.lineTo(px + cell, py + 0.5); ctx.stroke(); }
    if (x === 0) { ctx.beginPath(); ctx.moveTo(px + 0.5, py); ctx.lineTo(px + 0.5, py + cell); ctx.stroke(); }
  }
  ctx.strokeRect(ox, oy, total, total);
}

function drawCell(ctx, v, px, py, cell, conflict) {
  ctx.fillStyle = PALETTE.glass;
  ctx.fillRect(px, py, cell, cell);
  if (v === 2) {                             // WATER
    ctx.fillStyle = PALETTE.waterDeep;
    ctx.fillRect(px + 1, py + 2, cell - 2, cell - 2);
    ctx.fillStyle = PALETTE.water;
    ctx.fillRect(px + 2, py + 3, cell - 4, cell - 5);
    // Crest line at the top of the water.
    ctx.fillStyle = PALETTE.waterCrest;
    ctx.fillRect(px + 3, py + 3, cell - 6, 1);
    // A bubble.
    ctx.fillStyle = PALETTE.bubble;
    ctx.fillRect(px + cell - 6, py + 6, 2, 2);
  } else if (v === 1) {                      // AIR
    ctx.fillStyle = PALETTE.air;
    ctx.fillRect(px + 2, py + 2, cell - 4, cell - 4);
    ctx.fillStyle = PALETTE.airShade;
    ctx.fillRect(px + 4, py + cell - 6, 3, 1);
  }
  if (conflict) {
    ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
    ctx.fillRect(px, py, cell, cell);
  }
}

function drawHud(ctx, lang, p, marks, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (p.levelIndex + 1) + ' ' + p.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  const water = marks.filter(m => m === 2).length;
  const target = p.rc.reduce((a, b) => a + b, 0);
  ctx.fillText('WATER ' + water + '/' + target, 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
