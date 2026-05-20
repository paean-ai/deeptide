// Pixel-art rendering for Pixel Frogger. 360x480 world units.

const PALETTE = {
  bgRoad:    '#2d2d2d',
  bgLane:    '#383838',
  laneLine:  '#e8d76a',
  bgMedian:  '#3e5e2e',
  bgMedHi:   '#4c7236',
  bgBank:    '#5c8a3b',
  bgWater:   '#2952b6',
  bgWaterHi: '#3a6cd6',
  goalPad:   '#1e3a72',
  goalRing:  '#5fc0ff',
  goalFill:  '#5fc06e',
  log:       '#7a4a1f',
  logHi:     '#a06a3a',
  logLo:     '#4a2a0f',
  turtle:    '#3b7f3b',
  turtleHi:  '#5fa05f',
  turtleSh:  '#1f4d1f',
  carRed:    '#e85a3a',
  carRedHi:  '#ff8a6a',
  carBlue:   '#5a9aff',
  carBlueHi: '#82c0ff',
  carGreen:  '#7fc04a',
  carGreenHi:'#bce088',
  carYellow: '#f0c540',
  carYellowHi:'#ffe07a',
  truck:     '#7a5a3a',
  truckHi:   '#a68660',
  truckCab:  '#aa3a3a',
  frog:      '#7fd84a',
  frogHi:    '#bce088',
  frogLo:    '#2a5a14',
  frogEye:   '#fff7d0',
  frogPup:   '#0a0a0a',
  border:    '#0a0a0a',
  hud:       '#0a0a0a',
  hudText:   '#f8f5e8',
  hudDim:    '#a0a8b8',
  heart:     '#ff4a5a',
  warn:      '#f04a3a',
  pad:       '#f7c93a',
};

const TILE_W = 40;
const TILE_H = 32;

function cellRect(col, row) {
  return { x: BOARD_OX + col * TILE_W, y: BOARD_OY + row * TILE_H, w: TILE_W, h: TILE_H };
}
function cellRectF(colF, row) {
  return { x: BOARD_OX + colF * TILE_W, y: BOARD_OY + row * TILE_H, w: TILE_W, h: TILE_H };
}

function drawBackdrop(ctx) {
  // Sky/HUD strip at top.
  ctx.fillStyle = '#0a0a0a';
  ctx.fillRect(0, 0, VW, BOARD_OY);
  // Per-row backdrop tiles.
  for (let r = 0; r < ROWS; r++) {
    let bg, mark;
    if (r === ROW_GOAL)            { bg = PALETTE.bgBank; mark = null; }
    else if (r === ROW_BANK)       { bg = PALETTE.bgBank; mark = null; }
    else if (RIVER_ROWS.includes(r)) { bg = PALETTE.bgWater; mark = 'water'; }
    else if (r === ROW_MEDIAN)     { bg = PALETTE.bgMedian; mark = 'median'; }
    else if (ROAD_ROWS.includes(r)) { bg = PALETTE.bgRoad; mark = 'road'; }
    else                           { bg = PALETTE.bgBank; mark = null; }
    ctx.fillStyle = bg;
    ctx.fillRect(0, BOARD_OY + r * TILE_H, VW, TILE_H);
    if (mark === 'water') {
      // a couple of pixel-water highlight stripes
      ctx.fillStyle = PALETTE.bgWaterHi;
      for (let i = 0; i < VW; i += 8) {
        ctx.fillRect(i, BOARD_OY + r * TILE_H + 6, 4, 1);
        ctx.fillRect(i + 4, BOARD_OY + r * TILE_H + 22, 3, 1);
      }
    } else if (mark === 'median') {
      ctx.fillStyle = PALETTE.bgMedHi;
      for (let i = 0; i < VW; i += 14) {
        ctx.fillRect(i, BOARD_OY + r * TILE_H + 10, 6, 1);
      }
    } else if (mark === 'road') {
      // dashed lane line above (between adjacent road rows)
      if (r > ROAD_ROWS[0]) {
        ctx.fillStyle = PALETTE.laneLine;
        for (let i = 0; i < VW; i += 14) {
          ctx.fillRect(i, BOARD_OY + r * TILE_H - 1, 8, 2);
        }
      }
    }
  }
  // Border around playfield.
  ctx.fillStyle = PALETTE.border;
  ctx.fillRect(0, BOARD_OY, VW, 1);
  ctx.fillRect(0, BOARD_OY + ROWS * TILE_H - 1, VW, 1);
}

function drawGoalRow(ctx, pads) {
  // The five pads cut out of the bank.
  for (let i = 0; i < GOAL_COLS.length; i++) {
    const r = cellRect(GOAL_COLS[i], ROW_GOAL);
    ctx.fillStyle = PALETTE.goalPad;
    ctx.fillRect(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
    if (pads[i]) {
      // a sitting trophy frog
      drawFrog(ctx, r.x + r.w / 2, r.y + r.h / 2, 0, true, 0);
    } else {
      ctx.fillStyle = PALETTE.goalRing;
      ctx.fillRect(r.x + 4, r.y + 4, r.w - 8, 2);
      ctx.fillRect(r.x + 4, r.y + r.h - 6, r.w - 8, 2);
    }
  }
}

function drawLane(ctx, lane) {
  const y = BOARD_OY + lane.row * TILE_H;
  for (const ent of lane.entities) {
    const x = BOARD_OX + ent.x * TILE_W;
    if (x + lane.length * TILE_W < -2 || x > VW + 2) continue;
    if (lane.kind === 'car') drawCar(ctx, x, y, lane.length, lane.dir, lane.row);
    else if (lane.kind === 'log') drawLog(ctx, x, y, lane.length);
    else if (lane.kind === 'turtle') drawTurtle(ctx, x, y, lane.length);
  }
}

function drawCar(ctx, x, y, len, dir, row) {
  const w = len * TILE_W, h = TILE_H;
  if (len >= 2) {
    // truck — brown cab + cargo
    ctx.fillStyle = PALETTE.truck;
    ctx.fillRect(x + 2, y + 4, w - 4, h - 8);
    ctx.fillStyle = PALETTE.truckHi;
    ctx.fillRect(x + 2, y + 4, w - 4, 2);
    // cab on leading side
    const cx = dir > 0 ? x + w - TILE_W : x;
    ctx.fillStyle = PALETTE.truckCab;
    ctx.fillRect(cx + 4, y + 4, TILE_W - 6, h - 8);
    // windshield
    ctx.fillStyle = '#0a1024';
    if (dir > 0) ctx.fillRect(cx + TILE_W - 10, y + 8, 4, h - 16);
    else         ctx.fillRect(cx + 6, y + 8, 4, h - 16);
    // wheels
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(x + 6, y + h - 5, 6, 3);
    ctx.fillRect(x + w - 12, y + h - 5, 6, 3);
    return;
  }
  // 1-cell car: tint by row to add visual rhythm
  const palettes = [
    [PALETTE.carRed, PALETTE.carRedHi],
    [PALETTE.carBlue, PALETTE.carBlueHi],
    [PALETTE.carGreen, PALETTE.carGreenHi],
    [PALETTE.carYellow, PALETTE.carYellowHi],
  ];
  const [body, hi] = palettes[row % palettes.length];
  ctx.fillStyle = body;
  ctx.fillRect(x + 3, y + 6, w - 6, h - 12);
  ctx.fillStyle = hi;
  ctx.fillRect(x + 3, y + 6, w - 6, 2);
  // windshield in the direction of travel
  ctx.fillStyle = '#0a1024';
  if (dir > 0) ctx.fillRect(x + w - 12, y + 10, 6, h - 20);
  else         ctx.fillRect(x + 6, y + 10, 6, h - 20);
  // wheels
  ctx.fillStyle = '#0a0a0a';
  ctx.fillRect(x + 5, y + h - 5, 6, 3);
  ctx.fillRect(x + w - 11, y + h - 5, 6, 3);
}

function drawLog(ctx, x, y, len) {
  const w = len * TILE_W, h = TILE_H;
  ctx.fillStyle = PALETTE.logLo;
  ctx.fillRect(x + 2, y + 6, w - 4, h - 12);
  ctx.fillStyle = PALETTE.log;
  ctx.fillRect(x + 2, y + 8, w - 4, h - 16);
  ctx.fillStyle = PALETTE.logHi;
  ctx.fillRect(x + 2, y + 8, w - 4, 2);
  // bark notches
  ctx.fillStyle = PALETTE.logLo;
  for (let i = 0; i < len; i++) {
    ctx.fillRect(x + 8 + i * TILE_W, y + 12, 2, h - 20);
    ctx.fillRect(x + 22 + i * TILE_W, y + 14, 2, h - 22);
  }
}

function drawTurtle(ctx, x, y, len) {
  const w = len * TILE_W, h = TILE_H;
  for (let i = 0; i < len; i++) {
    const cx = x + i * TILE_W + TILE_W / 2;
    const cy = y + h / 2;
    ctx.fillStyle = PALETTE.turtleSh;
    fillDisk(ctx, cx, cy + 1, 12);
    ctx.fillStyle = PALETTE.turtle;
    fillDisk(ctx, cx, cy, 11);
    ctx.fillStyle = PALETTE.turtleHi;
    fillDisk(ctx, cx - 2, cy - 2, 6);
    ctx.fillStyle = PALETTE.turtleSh;
    ctx.fillRect(cx - 1, cy - 1, 3, 3);
  }
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}

// Frog. face: 0=up, 1=right, 2=left, 3=down. Hop is a 0..1 lift fraction.
function drawFrog(ctx, cx, cy, face, sitting, hop) {
  const lift = sitting ? 0 : hop * 4;
  const y = cy - lift;
  ctx.fillStyle = PALETTE.frogLo;
  fillDisk(ctx, cx, y + 2, 11);
  ctx.fillStyle = PALETTE.frog;
  fillDisk(ctx, cx, y, 10);
  ctx.fillStyle = PALETTE.frogHi;
  fillDisk(ctx, cx - 2, y - 2, 5);
  // eyes (pair offset slightly by facing)
  const dx = face === 1 ? 1 : face === 2 ? -1 : 0;
  const dy = face === 0 ? -1 : face === 3 ? 1 : 0;
  ctx.fillStyle = PALETTE.frogEye;
  ctx.fillRect(cx - 5 + dx, y - 6 + dy, 3, 3);
  ctx.fillRect(cx + 2 + dx, y - 6 + dy, 3, 3);
  ctx.fillStyle = PALETTE.frogPup;
  ctx.fillRect(cx - 4 + dx, y - 5 + dy, 1, 1);
  ctx.fillRect(cx + 3 + dx, y - 5 + dy, 1, 1);
  // legs
  ctx.fillStyle = PALETTE.frogLo;
  if (face === 0 || face === 3) {
    ctx.fillRect(cx - 8, y + 4, 3, 4);
    ctx.fillRect(cx + 5, y + 4, 3, 4);
  } else {
    ctx.fillRect(cx - 7, y + 5, 4, 3);
    ctx.fillRect(cx + 3, y + 5, 4, 3);
  }
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, BOARD_OY);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.lv.name[0], 6, 16);
  // lives as hearts
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) {
    drawHeart(ctx, 86 + i * 12, 16);
  }
  ctx.textAlign = 'center';
  ctx.fillStyle = s.timeLeft < 10 ? PALETTE.warn : PALETTE.hudText;
  ctx.fillText(t(lang, 'time') + ' ' + Math.max(0, Math.ceil(s.timeLeft)), VW / 2, 16);
  ctx.fillStyle = PALETTE.hudText;
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 16);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.35);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.frog.alive ? `rgba(95,192,110,${0.35 * a})` :
                                 `rgba(255,80,80,${0.5 * a})`;
  ctx.fillRect(0, BOARD_OY, VW, ROWS * TILE_H);
}
