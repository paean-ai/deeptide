// Pixel-art rendering for Pixel Hanoi. 360x480 world units.

const PALETTE = {
  bg:        '#1a1224',
  bgHi:      '#241936',
  ground:    '#3a2a1a',
  groundHi:  '#5c422a',
  groundLo:  '#1c130a',
  peg:       '#a07a3a',
  pegHi:     '#d8a560',
  pegLo:     '#5a4014',
  border:    '#070315',
  hud:       '#070315',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  select:    '#f4d27b',
  star:      '#f8d34a',
  starOff:   '#3a4274',
  win:       '#5fc06e',
  // Disk colour ramp (largest disk first, then descending sizes).
  // Eight slots so an 8-disk tower has a unique colour per disk.
  disks: [
    '#ff5a5a', '#ff9b40', '#f4d27b',
    '#5fc06e', '#5fc0ff', '#9a6cd8',
    '#ff8fd0', '#bda6ff',
  ],
  diskHi: [
    '#ffa090', '#ffc080', '#fff0c8',
    '#86df9d', '#82c0ff', '#cdaee8',
    '#ffc4ec', '#e3d3ff',
  ],
};

// Geometry: three pegs spaced across a 360 px frame. The ground band sits
// near the bottom; the peg poles rise from it.
const GROUND_Y = 380;
const PEG_H = 200;
const PEG_W = 6;
const PEG_X = [80, 180, 280];

function drawBackdrop(ctx) {
  // Vertical gradient + faint speckle.
  const grad = ctx.createLinearGradient(0, 0, 0, VH);
  grad.addColorStop(0, PALETTE.bg);
  grad.addColorStop(1, PALETTE.bgHi);
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = '#2a1d3a';
  for (let i = 0; i < 28; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawTable(ctx) {
  // Ground band.
  ctx.fillStyle = PALETTE.groundLo;
  ctx.fillRect(0, GROUND_Y, VW, VH - GROUND_Y);
  ctx.fillStyle = PALETTE.ground;
  ctx.fillRect(0, GROUND_Y, VW, 8);
  ctx.fillStyle = PALETTE.groundHi;
  ctx.fillRect(0, GROUND_Y, VW, 2);
  // Pegs.
  for (const x of PEG_X) {
    ctx.fillStyle = PALETTE.pegLo;
    ctx.fillRect(x - PEG_W / 2 - 1, GROUND_Y - PEG_H, PEG_W + 2, PEG_H + 4);
    ctx.fillStyle = PALETTE.peg;
    ctx.fillRect(x - PEG_W / 2,     GROUND_Y - PEG_H, PEG_W, PEG_H);
    ctx.fillStyle = PALETTE.pegHi;
    ctx.fillRect(x - PEG_W / 2,     GROUND_Y - PEG_H, 1, PEG_H);
    // Cap at the top so disks look like they're threaded through.
    ctx.fillStyle = PALETTE.peg;
    ctx.fillRect(x - PEG_W / 2 - 2, GROUND_Y - PEG_H - 2, PEG_W + 4, 2);
  }
}

// Disk width in pixels for the given disk SIZE (1 = smallest) within an
// N-disk game. We taper from a generous bottom disk down to a peg-hugging
// smallest disk so the silhouette reads as a tidy pyramid.
function diskWidth(size, disks) {
  const maxW = 86;
  const minW = 22;
  const step = (maxW - minW) / Math.max(1, disks - 1);
  return Math.round(minW + step * (size - 1));
}

function drawDisks(ctx, s, animating) {
  const diskCount = s.disks;
  for (let p = 0; p < 3; p++) {
    const stack = s.pegs[p];
    for (let i = 0; i < stack.length; i++) {
      const sz = stack[i];
      const w = diskWidth(sz, diskCount);
      const y = GROUND_Y - 12 - i * 14;
      drawDisk(ctx, PEG_X[p], y, w, sz);
    }
    if (s.selected === p && stack.length) {
      // Lift the top disk slightly to show it's picked up.
      const sz = stack[stack.length - 1];
      const w = diskWidth(sz, diskCount);
      const y = GROUND_Y - 12 - (stack.length - 1) * 14 - 6;
      drawDisk(ctx, PEG_X[p], y, w, sz, true);
    }
  }
}

function drawDisk(ctx, cx, baseY, w, size, lifted) {
  const h = 12;
  const x = cx - w / 2;
  const idx = (size - 1) % PALETTE.disks.length;
  const body = PALETTE.disks[idx];
  const hi   = PALETTE.diskHi[idx];
  ctx.fillStyle = PALETTE.border;
  ctx.fillRect(x, baseY - h, w, h);
  ctx.fillStyle = body;
  ctx.fillRect(x + 1, baseY - h + 1, w - 2, h - 2);
  ctx.fillStyle = hi;
  ctx.fillRect(x + 1, baseY - h + 1, w - 2, 2);
  // Centre hole through which the peg passes — let the peg show through
  // by clearing a strip in the middle.
  ctx.fillStyle = PALETTE.peg;
  ctx.fillRect(cx - PEG_W / 2, baseY - h, PEG_W, h);
  // Lift highlight ring.
  if (lifted) {
    ctx.strokeStyle = PALETTE.select;
    ctx.lineWidth = 2;
    ctx.strokeRect(x - 1, baseY - h - 1, w + 2, h + 2);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'moves') + ' ' + s.moves + ' / ' + t(lang, 'par') + ' ' + par(s.disks), VW / 2, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'best') + ' ' + (best || '—'), VW - 6, 16);
}

function drawStars(ctx, x, y, n, w = 14) {
  for (let i = 0; i < 3; i++) {
    ctx.fillStyle = i < n ? PALETTE.star : PALETTE.starOff;
    drawStar(ctx, x + i * (w + 4) + w / 2, y, w / 2);
  }
}
function drawStar(ctx, cx, cy, r) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const a = (Math.PI / 5) * i - Math.PI / 2;
    const rr = i % 2 === 0 ? r : r * 0.45;
    ctx.lineTo(cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
  }
  ctx.closePath();
  ctx.fill();
}
