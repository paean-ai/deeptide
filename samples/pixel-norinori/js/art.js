// Pixel-art rendering for Pixel Norinori. 360x480 world units.

const REGION_TINTS = [
  '#2a3a6c', '#3a3a72', '#2a5470', '#3a5a4a',
  '#5a4a2a', '#5a3a52', '#465c2a', '#2a4a5a',
];

const PALETTE = {
  bg:        '#0d1228',
  cellHi:    '#3a4274',
  shaded:    '#0c1230',
  shadedTop: '#1c2240',
  dot:       '#9aa6cc',
  hint:      '#f7e69a',
  hintShaded:'#bd9e3a',
  border:    '#07091a',
  regEdge:   '#f8f5e8',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  conflict:  '#e8554f',
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

function drawGrid(ctx, n, regions, hints, marks, conflicts) {
  const { cell, total, ox, oy } = gridGeometry(n);
  const owner = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const px = ox + x * cell, py = oy + y * cell;
    drawCell(ctx, regions, hints, marks, conflicts, x, y, i, owner, px, py, cell);
  }
  // Grid + region borders.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= n; i++) {
    ctx.beginPath(); ctx.moveTo(ox + i * cell + 0.5, oy); ctx.lineTo(ox + i * cell + 0.5, oy + total); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(ox, oy + i * cell + 0.5); ctx.lineTo(ox + total, oy + i * cell + 0.5); ctx.stroke();
  }
  ctx.strokeStyle = PALETTE.regEdge;
  ctx.lineWidth = 2;
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const px = ox + x * cell, py = oy + y * cell;
    const reg = owner[i];
    if (x === n - 1 || owner[y * n + x + 1] !== reg) {
      ctx.beginPath(); ctx.moveTo(px + cell + 0.5, py); ctx.lineTo(px + cell + 0.5, py + cell); ctx.stroke();
    }
    if (y === n - 1 || owner[(y + 1) * n + x] !== reg) {
      ctx.beginPath(); ctx.moveTo(px, py + cell + 0.5); ctx.lineTo(px + cell, py + cell + 0.5); ctx.stroke();
    }
    if (y === 0) { ctx.beginPath(); ctx.moveTo(px, py + 0.5); ctx.lineTo(px + cell, py + 0.5); ctx.stroke(); }
    if (x === 0) { ctx.beginPath(); ctx.moveTo(px + 0.5, py); ctx.lineTo(px + 0.5, py + cell); ctx.stroke(); }
  }
  ctx.strokeRect(ox, oy, total, total);
}

function drawCell(ctx, regions, hints, marks, conflicts, x, y, i, owner, px, py, cell) {
  // Base tint = region colour.
  ctx.fillStyle = REGION_TINTS[owner[i] % REGION_TINTS.length];
  ctx.fillRect(px, py, cell, cell);
  // Mark / hint render.
  const isHint = hints[i] >= 0;
  const v = marks[i];
  if (v === 1 || (isHint && hints[i] === 1)) {
    // Shaded.
    ctx.fillStyle = PALETTE.shadedTop;
    ctx.fillRect(px + 1, py + 1, cell - 2, cell - 2);
    ctx.fillStyle = PALETTE.shaded;
    ctx.fillRect(px + 2, py + 3, cell - 4, cell - 5);
  } else if (v === 0 || (isHint && hints[i] === 0)) {
    // Empty mark (dot).
    ctx.fillStyle = PALETTE.dot;
    ctx.fillRect((px + cell / 2 | 0) - 2, (py + cell / 2 | 0) - 2, 4, 4);
  }
  // Hint corner tag.
  if (isHint) {
    ctx.fillStyle = hints[i] === 1 ? PALETTE.hintShaded : PALETTE.hint;
    ctx.fillRect(px + cell - 5, py + 1, 4, 4);
  }
  // Conflict tint overlay.
  if (conflicts && conflicts.has(i)) {
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
  const shaded = marks.filter(v => v === 1).length;
  const target = p.regions.length * 2;
  ctx.fillText(shaded + '/' + target + ' SHADED', 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
