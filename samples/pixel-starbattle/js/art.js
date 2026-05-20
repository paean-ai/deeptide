// Pixel-art rendering for Pixel Star Battle. 360x480 world units.

// Distinct region tints to make the partition pop. Six palette slots used
// modulo n - regions still get unique borders even when two share a colour.
const REGION_TINTS = [
  '#2a3a6c', '#3a3a72', '#2a5470', '#3a5a4a',
  '#5a4a2a', '#5a3a52', '#465c2a', '#2a4a5a',
];

const PALETTE = {
  bg:       '#0d1228',
  hud:      '#0d1228',
  hudText:  '#f8f5e8',
  hudDim:   '#9aa6cc',
  border:   '#0c1230',
  regEdge:  '#f8f5e8',
  star:     '#f7e69a',
  starGlow: '#fffaca',
  cross:    '#9aa6cc',
  conflict: '#e8554f',
  ok:       '#54c47c',
};

function gridGeometry(n) {
  const size = 320;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 64;
  return { cell, total, ox, oy };
}

function drawGrid(ctx, n, regions, marks, conflicts) {
  const { cell, ox, oy, total } = gridGeometry(n);
  const owner = new Array(n * n);
  regions.forEach((r, id) => r.forEach(c => owner[c] = id));
  // Cell tints.
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const px = ox + x * cell, py = oy + y * cell;
    ctx.fillStyle = REGION_TINTS[owner[i] % REGION_TINTS.length];
    ctx.fillRect(px, py, cell, cell);
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
      ctx.fillRect(px, py, cell, cell);
    }
    // Cell marks.
    if (marks[i] === 1) drawStar(ctx, px + cell / 2, py + cell / 2, cell);
    else if (marks[i] === 2) drawCross(ctx, px + cell / 2, py + cell / 2, cell);
  }
  // Thin grid lines.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= n; i++) {
    ctx.beginPath();
    ctx.moveTo(ox + i * cell + 0.5, oy);
    ctx.lineTo(ox + i * cell + 0.5, oy + total);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(ox,         oy + i * cell + 0.5);
    ctx.lineTo(ox + total, oy + i * cell + 0.5);
    ctx.stroke();
  }
  // Thick region borders.
  ctx.strokeStyle = PALETTE.regEdge;
  ctx.lineWidth = 2;
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const px = ox + x * cell, py = oy + y * cell;
    const reg = owner[i];
    if (x === n - 1 || owner[y * n + x + 1] !== reg) {
      ctx.beginPath();
      ctx.moveTo(px + cell + 0.5, py);
      ctx.lineTo(px + cell + 0.5, py + cell);
      ctx.stroke();
    }
    if (y === n - 1 || owner[(y + 1) * n + x] !== reg) {
      ctx.beginPath();
      ctx.moveTo(px,         py + cell + 0.5);
      ctx.lineTo(px + cell,  py + cell + 0.5);
      ctx.stroke();
    }
    if (y === 0) { ctx.beginPath(); ctx.moveTo(px, py + 0.5); ctx.lineTo(px + cell, py + 0.5); ctx.stroke(); }
    if (x === 0) { ctx.beginPath(); ctx.moveTo(px + 0.5, py); ctx.lineTo(px + 0.5, py + cell); ctx.stroke(); }
  }
  ctx.strokeRect(ox, oy, total, total);
}

function drawStar(ctx, cx, cy, cell) {
  // Pixel 5-point star within the cell.
  const r = (cell * 0.38) | 0;
  ctx.fillStyle = PALETTE.starGlow;
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const ang = -Math.PI / 2 + (i * Math.PI) / 5;
    const rad = i % 2 === 0 ? r : (r * 0.45);
    const px = cx + Math.cos(ang) * rad;
    const py = cy + Math.sin(ang) * rad;
    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
  }
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = PALETTE.star;
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const ang = -Math.PI / 2 + (i * Math.PI) / 5;
    const rad = (i % 2 === 0 ? r : (r * 0.45)) * 0.85;
    const px = cx + Math.cos(ang) * rad;
    const py = cy + Math.sin(ang) * rad;
    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
  }
  ctx.closePath();
  ctx.fill();
}

function drawCross(ctx, cx, cy, cell) {
  const r = (cell * 0.18) | 0;
  ctx.strokeStyle = PALETTE.cross;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cx - r, cy - r); ctx.lineTo(cx + r, cy + r);
  ctx.moveTo(cx + r, cy - r); ctx.lineTo(cx - r, cy + r);
  ctx.stroke();
}

function drawHud(ctx, lang, levelIndex, cfg, marks, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (levelIndex + 1) + ' ' + cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  const stars = marks.filter(m => m === 1).length;
  ctx.fillText(`${stars}/${cfg.n * cfg.k} ★`, 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(`${t(lang, 'timeStr')} ${min}:${sec.toString().padStart(2,'0')}`, 352, 16);
}
