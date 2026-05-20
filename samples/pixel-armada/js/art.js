// Pixel-art rendering for Pixel Armada. 360x480 world units.

const PALETTE = {
  bg:       '#0f1535',
  card:     '#1c2552',
  sea:      '#21407a',
  seaShade: '#2c5294',
  ship:     '#9aa6cc',
  shipDark: '#3a4576',
  shipHi:   '#dde6ff',
  water:    '#79b7e5',
  waterX:   '#5793c7',
  hint:     '#f7e69a',
  conflict: '#e8554f',
  border:   '#07091a',
  hud:      '#0d1228',
  hudText:  '#f8f5e8',
  hudDim:   '#9aa6cc',
  ok:       '#54c47c',
};

function gridGeometry(n) {
  // 320px square board (with 30 px margin for count labels), below the HUD.
  const size = 280;
  const cell = (size / n) | 0;
  const total = cell * n;
  const ox = ((360 - total) / 2) | 0;
  const oy = 72;
  return { cell, total, ox, oy };
}

function drawScene(ctx, p, marks, conflicts) {
  const { cell, ox, oy, total } = gridGeometry(p.n);
  // Column counts (above the board).
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold ' + ((cell * 0.5) | 0) + 'px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let x = 0; x < p.n; x++) {
    const px = ox + x * cell + cell / 2;
    ctx.fillText(String(p.cc[x]), px, oy - 12);
  }
  // Row counts (left of the board).
  for (let y = 0; y < p.n; y++) {
    const py = oy + y * cell + cell / 2;
    ctx.fillText(String(p.rc[y]), ox - 14, py);
  }
  // Cells.
  for (let y = 0; y < p.n; y++) for (let x = 0; x < p.n; x++) {
    const i = y * p.n + x;
    const px = ox + x * cell, py = oy + y * cell;
    drawCell(ctx, p, marks, conflicts, x, y, px, py, cell);
  }
  // Grid lines.
  ctx.strokeStyle = PALETTE.border;
  ctx.lineWidth = 1;
  for (let i = 0; i <= p.n; i++) {
    ctx.beginPath(); ctx.moveTo(ox + i * cell + 0.5, oy); ctx.lineTo(ox + i * cell + 0.5, oy + total); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(ox, oy + i * cell + 0.5); ctx.lineTo(ox + total, oy + i * cell + 0.5); ctx.stroke();
  }
  ctx.lineWidth = 2;
  ctx.strokeRect(ox, oy, total, total);
}

function drawCell(ctx, p, marks, conflicts, x, y, px, py, cell) {
  const i = y * p.n + x;
  const v = marks[i];
  const isHint = p.hints[i] >= 0;
  ctx.fillStyle = PALETTE.sea;
  ctx.fillRect(px, py, cell, cell);
  if (v === 1) {
    drawShip(ctx, p, marks, x, y, px, py, cell, isHint);
  } else if (v === 0) {
    drawWater(ctx, px, py, cell, isHint);
  }
  if (conflicts && conflicts.has(i)) {
    ctx.fillStyle = 'rgba(232, 85, 79, 0.45)';
    ctx.fillRect(px, py, cell, cell);
  }
  if (isHint) {
    // Small corner tag for the hint cell.
    ctx.fillStyle = PALETTE.hint;
    ctx.fillRect(px + cell - 5, py + 1, 4, 4);
  }
}

// Render a ship cell based on its neighbours: end / middle / single.
function drawShip(ctx, p, marks, x, y, px, py, cell, isHint) {
  const n = p.n;
  const i = y * n + x;
  const leftShip  = x > 0     && marks[i - 1] === 1;
  const rightShip = x < n - 1 && marks[i + 1] === 1;
  const upShip    = y > 0     && marks[i - n] === 1;
  const downShip  = y < n - 1 && marks[i + n] === 1;
  const horiz = leftShip || rightShip;
  const vert  = upShip || downShip;
  ctx.fillStyle = PALETTE.shipDark;
  ctx.fillRect(px + 1, py + 1, cell - 2, cell - 2);
  ctx.fillStyle = isHint ? PALETTE.hint : PALETTE.ship;
  // Body inset.
  const pad = 2;
  ctx.fillRect(px + pad, py + pad, cell - pad * 2, cell - pad * 2);
  // Highlight strip — different shape per ship segment type.
  ctx.fillStyle = PALETTE.shipHi;
  if (!horiz && !vert) {
    // Single-cell submarine - a small dot in the middle.
    ctx.fillRect(px + (cell / 2 | 0) - 1, py + (cell / 2 | 0) - 1, 3, 3);
  } else if (horiz) {
    // Horizontal stripe across the middle.
    ctx.fillRect(px + 3, py + (cell / 2 | 0) - 1, cell - 6, 2);
  } else {
    ctx.fillRect(px + (cell / 2 | 0) - 1, py + 3, 2, cell - 6);
  }
  // End caps.
  if (horiz && !leftShip)  drawCap(ctx, px + 1, py, cell, 'L');
  if (horiz && !rightShip) drawCap(ctx, px + 1, py, cell, 'R');
  if (vert  && !upShip)    drawCap(ctx, px, py + 1, cell, 'U');
  if (vert  && !downShip)  drawCap(ctx, px, py + 1, cell, 'D');
}
function drawCap(ctx, x, y, cell, dir) {
  ctx.fillStyle = PALETTE.shipDark;
  if (dir === 'L') ctx.fillRect(x, y + 3, 2, cell - 6);
  if (dir === 'R') ctx.fillRect(x + cell - 3, y + 3, 2, cell - 6);
  if (dir === 'U') ctx.fillRect(x + 3, y, cell - 6, 2);
  if (dir === 'D') ctx.fillRect(x + 3, y + cell - 3, cell - 6, 2);
}

function drawWater(ctx, px, py, cell, isHint) {
  // Wavy water; if it's a hint, use a stronger blue, otherwise dim.
  ctx.fillStyle = isHint ? PALETTE.water : PALETTE.waterX;
  ctx.fillRect(px + 2, py + 2, cell - 4, cell - 4);
  ctx.fillStyle = isHint ? '#cfe3ff' : '#7ba3cc';
  ctx.fillRect(px + 4, py + (cell / 2 | 0), 3, 1);
  ctx.fillRect(px + cell - 7, py + (cell / 2 | 0) + 2, 3, 1);
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
  const ships = marks.filter(m => m === 1).length;
  const target = p.fleet.reduce((a, b) => a + b, 0);
  ctx.fillText('SHIP ' + ships + '/' + target, 180, 16);
  ctx.textAlign = 'right';
  const min = (elapsedSec / 60) | 0, sec = (elapsedSec % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}

// Fleet card at the bottom — shows the ships and their sizes.
function drawFleetCard(ctx, lang, p, marks) {
  const top = 388, h = 50;
  ctx.fillStyle = PALETTE.card;
  ctx.fillRect(8, top, 344, h);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 10px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  ctx.fillText(t(lang, 'fleet') + ':', 14, top + 4);
  // Lay out the ships left to right.
  let x = 14;
  const y = top + 22;
  for (const size of p.fleet) {
    for (let k = 0; k < size; k++) {
      ctx.fillStyle = PALETTE.ship;
      ctx.fillRect(x + k * 12, y, 10, 14);
      ctx.fillStyle = PALETTE.shipHi;
      ctx.fillRect(x + k * 12 + 3, y + 6, 4, 2);
    }
    x += size * 12 + 8;
  }
}
