// Pixel Cycle - rendering + on-screen controls.

const COL = {
  field: '#0a1224', fieldEdge: '#2a3b5c', grid: '#15233e',
  player: '#7fffd4', playerTrail: '#3fa897',
  cpu: '#ff6e7a', cpuTrail: '#a83947',
  pad: '#2e3858', padOn: '#ffe07a', padIco: '#e6ebf5',
  flash: 'rgba(255,255,255,0.18)',
};

const PAD_L = { x: 16,  y: 410, w: 130, h: 56, key: 'left' };
const PAD_R = { x: 214, y: 410, w: 130, h: 56, key: 'right' };

function drawBackground(ctx) {
  ctx.fillStyle = '#04060e';
  ctx.fillRect(0, 0, VW, VH);
}

function drawField(ctx, s) {
  // field background
  ctx.fillStyle = COL.field;
  ctx.fillRect(BOARD_X - 2, BOARD_Y - 2, GRID_W * CELL + 4, GRID_H * CELL + 4);
  ctx.strokeStyle = COL.fieldEdge;
  ctx.lineWidth = 2;
  ctx.strokeRect(BOARD_X - 1.5, BOARD_Y - 1.5, GRID_W * CELL + 3, GRID_H * CELL + 3);
  // grid lines (subtle)
  ctx.strokeStyle = COL.grid;
  ctx.lineWidth = 1;
  for (let x = 0; x <= GRID_W; x += 5) {
    const xx = BOARD_X + x * CELL + 0.5;
    ctx.beginPath(); ctx.moveTo(xx, BOARD_Y); ctx.lineTo(xx, BOARD_Y + GRID_H * CELL); ctx.stroke();
  }
  for (let y = 0; y <= GRID_H; y += 5) {
    const yy = BOARD_Y + y * CELL + 0.5;
    ctx.beginPath(); ctx.moveTo(BOARD_X, yy); ctx.lineTo(BOARD_X + GRID_W * CELL, yy); ctx.stroke();
  }
  // trails
  for (let y = 0; y < GRID_H; y++) {
    for (let x = 0; x < GRID_W; x++) {
      const v = s.grid[y * GRID_W + x];
      if (!v) continue;
      ctx.fillStyle = v === 1 ? COL.playerTrail : COL.cpuTrail;
      ctx.fillRect(BOARD_X + x * CELL + 2, BOARD_Y + y * CELL + 2, CELL - 4, CELL - 4);
    }
  }
  // cycle heads
  drawHead(ctx, s.player, COL.player);
  drawHead(ctx, s.cpu, COL.cpu);
  // round-over flash
  if (s.roundOver && !s.over) {
    ctx.fillStyle = COL.flash;
    ctx.fillRect(BOARD_X, BOARD_Y, GRID_W * CELL, GRID_H * CELL);
  }
}

function drawHead(ctx, cycle, col) {
  if (!cycle.alive) return;
  const x = BOARD_X + cycle.x * CELL, y = BOARD_Y + cycle.y * CELL;
  ctx.fillStyle = col;
  ctx.fillRect(x, y, CELL, CELL);
  // direction pip
  ctx.fillStyle = '#0a1224';
  const cx = x + CELL / 2, cy = y + CELL / 2;
  const d = DIRS[cycle.d];
  ctx.fillRect(cx + d.dx * 2 - 1.5, cy + d.dy * 2 - 1.5, 3, 3);
}

function drawControls(ctx, pressed) {
  for (const b of [PAD_L, PAD_R]) {
    ctx.fillStyle = pressed[b.key] ? COL.padOn : COL.pad;
    ctx.fillRect(b.x, b.y, b.w, b.h);
    ctx.strokeStyle = '#1a1a26';
    ctx.lineWidth = 2;
    ctx.strokeRect(b.x + 1, b.y + 1, b.w - 2, b.h - 2);
    ctx.fillStyle = COL.padIco;
    const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
    ctx.beginPath();
    if (b.key === 'left') {
      ctx.moveTo(cx - 14, cy); ctx.lineTo(cx + 10, cy - 14); ctx.lineTo(cx + 10, cy + 14);
    } else {
      ctx.moveTo(cx + 14, cy); ctx.lineTo(cx - 10, cy - 14); ctx.lineTo(cx - 10, cy + 14);
    }
    ctx.closePath();
    ctx.fill();
  }
}
