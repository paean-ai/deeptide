// Pixel-art rendering for Pixel Marble. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  floorA:    '#262d54',
  floorB:    '#1f2548',
  wallTop:   '#9aa6cc',
  wallSide:  '#5a6188',
  wallDark:  '#3a4274',
  holeRim:   '#3a2a18',
  holeInner: '#070b16',
  goal:      '#54c47c',
  goalEdge:  '#2b6f3a',
  marble:    '#f8f5e8',
  marbleDark:'#9aa6cc',
  marbleSheen:'#fff',
  shadow:    'rgba(0,0,0,0.35)',
  arrow:     '#f7e69a',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function drawScene(ctx, s, lang) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, 360, 480);
  drawField(ctx, s);
  drawWalls(ctx, s);
  drawHolesGoal(ctx, s);
  drawBall(ctx, s);
  drawTiltArrow(ctx, s);
  drawHud(ctx, s, lang);
}

function drawField(ctx, s) {
  // Checker-tile floor for visual interest.
  for (let y = 0; y < GRID_H; y++) for (let x = 0; x < GRID_W; x++) {
    const t = s.grid[y * GRID_W + x];
    if (t === WALL) continue;
    const px = PLAY_X + x * CELL, py = PLAY_Y + y * CELL;
    ctx.fillStyle = ((x + y) & 1) ? PALETTE.floorA : PALETTE.floorB;
    ctx.fillRect(px, py, CELL, CELL);
  }
}

function drawWalls(ctx, s) {
  for (let y = 0; y < GRID_H; y++) for (let x = 0; x < GRID_W; x++) {
    if (s.grid[y * GRID_W + x] !== WALL) continue;
    const px = PLAY_X + x * CELL, py = PLAY_Y + y * CELL;
    ctx.fillStyle = PALETTE.wallDark;
    ctx.fillRect(px, py, CELL, CELL);
    ctx.fillStyle = PALETTE.wallSide;
    ctx.fillRect(px + 1, py + 1, CELL - 2, CELL - 2);
    ctx.fillStyle = PALETTE.wallTop;
    ctx.fillRect(px + 2, py + 2, CELL - 4, 4);
  }
}

function drawHolesGoal(ctx, s) {
  for (let y = 0; y < GRID_H; y++) for (let x = 0; x < GRID_W; x++) {
    const t = s.grid[y * GRID_W + x];
    if (t !== HOLE && t !== GOAL) continue;
    const px = PLAY_X + x * CELL + CELL / 2;
    const py = PLAY_Y + y * CELL + CELL / 2;
    if (t === HOLE) {
      ctx.fillStyle = PALETTE.holeRim;
      ctx.beginPath(); ctx.arc(px, py, 8, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = PALETTE.holeInner;
      ctx.beginPath(); ctx.arc(px, py, 6, 0, Math.PI * 2); ctx.fill();
    } else {
      ctx.fillStyle = PALETTE.goalEdge;
      ctx.fillRect(px - 8, py - 8, 16, 16);
      ctx.fillStyle = PALETTE.goal;
      ctx.fillRect(px - 7, py - 7, 14, 14);
      ctx.fillStyle = PALETTE.goalEdge;
      ctx.fillRect(px - 6, py - 1, 12, 2);
      ctx.fillRect(px - 1, py - 6, 2, 12);
    }
  }
}

function drawBall(ctx, s) {
  const b = s.ball;
  if (!b.alive) return;
  const x = PLAY_X + b.x, y = PLAY_Y + b.y;
  ctx.fillStyle = PALETTE.shadow;
  ctx.beginPath(); ctx.arc(x + 1, y + 2, BALL_R + 1, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.marbleDark;
  ctx.beginPath(); ctx.arc(x, y, BALL_R + 1, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.marble;
  ctx.beginPath(); ctx.arc(x, y, BALL_R, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.marbleSheen;
  ctx.fillRect((x - 2) | 0, (y - 3) | 0, 2, 1);
}

function drawTiltArrow(ctx, s) {
  if (!s.tilt || (!s.tilt.x && !s.tilt.y)) return;
  const b = s.ball;
  if (!b.alive) return;
  const cx = PLAY_X + b.x, cy = PLAY_Y + b.y;
  const ux = s.tilt.x, uy = s.tilt.y;
  ctx.fillStyle = PALETTE.arrow;
  for (let d = 10; d < 30; d += 4) {
    ctx.fillRect((cx + ux * d - 1) | 0, (cy + uy * d - 1) | 0, 2, 2);
  }
}

function drawHud(ctx, s, lang) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'right';
  const min = (s.elapsed / 60) | 0, sec = (s.elapsed % 60) | 0;
  ctx.fillText(t(lang, 'timeStr') + ' ' + min + ':' + sec.toString().padStart(2, '0'), 352, 16);
}
