// Pixel-art rendering for Pixel Nurikabe. 360x480 world units.

const PALETTE = {
  bg: '#1d2240',
  card: '#262d54',
  cellBlank: '#e8e3c8',     // unmarked
  cellSea:   '#172548',     // shaded sea
  cellDot:   '#e8e3c8',     // marked-island ("definitely not sea") - shows a dot
  cellIsland:'#f8f5e8',     // when shown filled as island (solution view)
  border:    '#0c1230',
  clueText:  '#0c1230',
  clueBg:    '#f7e69a',
  dotColor:  '#7d8ba0',
  conflict:  '#e8554f',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function gridGeometry(n) {
  // Board sits below a 32px HUD. Uses 320 of the 360 width for n cells.
  const size = 320;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 64;
  return { cell, total, ox, oy };
}

function drawGrid(ctx, n, clues, grid, conflicts) {
  const { cell, ox, oy, total } = gridGeometry(n);
  // Each cell is one of UNKNOWN/SEA/dot (k+2 from data.js).
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const i = y * n + x;
    const v = grid[i];
    const px = ox + x * cell, py = oy + y * cell;
    let fill;
    if (v === 1)      fill = PALETTE.cellSea;       // SEA
    else if (v >= 2)  fill = PALETTE.cellDot;       // marked island
    else              fill = PALETTE.cellBlank;
    ctx.fillStyle = fill;
    ctx.fillRect(px, py, cell, cell);
    if (conflicts && conflicts.has(i)) {
      ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
      ctx.fillRect(px, py, cell, cell);
    }
    // White-dot mark when the player has tagged a non-clue cell as island.
    if (v >= 2 && !isClueCell(clues, x, y)) {
      ctx.fillStyle = PALETTE.dotColor;
      ctx.fillRect(px + cell / 2 - 2, py + cell / 2 - 2, 4, 4);
    }
  }
  // Clue cells.
  ctx.font = 'bold ' + ((cell * 0.55) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (const c of clues) {
    const px = ox + c.x * cell, py = oy + c.y * cell;
    ctx.fillStyle = PALETTE.clueBg;
    ctx.fillRect(px + 1, py + 1, cell - 2, cell - 2);
    ctx.fillStyle = PALETTE.clueText;
    ctx.fillText(String(c.size), px + cell / 2, py + cell / 2 + 1);
  }
  // Grid lines.
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
  // Thick outer border.
  ctx.lineWidth = 2;
  ctx.strokeRect(ox, oy, total, total);
}

function isClueCell(clues, x, y) {
  for (const c of clues) if (c.x === x && c.y === y) return true;
  return false;
}

function drawHud(ctx, lang, levelIndex, mistakes, elapsedSec) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText('L' + (levelIndex + 1), 8, 16);
  ctx.textAlign = 'center';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(`${t(lang, 'timeStr')} ${min}:${sec.toString().padStart(2,'0')}`, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillStyle = mistakes ? PALETTE.conflict : PALETTE.hudText;
  ctx.fillText(`${t(lang, 'mistakes')}: ${mistakes}`, 352, 16);
}
