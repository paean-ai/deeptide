// Pixel Frogger - cross five lanes of cars and a river of logs/turtles to
// fill all five goal pads at the top. Tap or swipe to hop one cell at a
// time. Each new level speeds the traffic up.
//
// Grid: 9 columns x 13 rows in cell units (40w x 32h each).
//   row 0:  goal row (5 pads, hop in to score)
//   row 1:  river bank (safe)
//   rows 2-5: river lanes (must ride a log/turtle; bare water = drown)
//   row 6:  median (safe)
//   rows 7-11: road lanes (cars; collision = death)
//   row 12: start row (safe)
//
// All entity positions live in cell-fraction (float) units; updates use a
// fixed sub-step inside the variable-dt tick to keep collisions stable.

const VW = 360, VH = 480;

const COLS = 9;
const ROWS = 13;
const CELL_W = 40;
const CELL_H = 32;
const BOARD_OX = 0;
const BOARD_OY = 32;          // HUD takes the top 32 px

const ROW_START  = 12;
const ROW_MEDIAN = 6;
const ROW_BANK   = 1;
const ROW_GOAL   = 0;
const ROAD_ROWS  = [7, 8, 9, 10, 11];
const RIVER_ROWS = [2, 3, 4, 5];

// 5 goal pads at columns 1, 3, 5 (centre), 7, and a wraparound at col... 0/8?
// Classic Frogger has 5 pads — let's place at columns 1, 2.5, 4, 5.5, 7 → use
// cols [1, 3, 4 (centre), 5, 7] for a 9-column board.
const GOAL_COLS = [1, 3, 4, 5, 7];
const GOAL_COUNT = GOAL_COLS.length;

// ---- levels ------------------------------------------------------------
// `lanes` maps row -> { kind: 'car' | 'log' | 'turtle', speed: cells/s,
// dir: +1 | -1, gap: cell spacing, length: cells (entity width) }.
// We hand-author one lane per row; level scaling multiplies speeds.

const BASE_ROAD = {
  7:  { kind: 'car', dir: -1, speed: 2.2, gap: 4.5, length: 1 },
  8:  { kind: 'car', dir: +1, speed: 3.4, gap: 5.5, length: 1 },
  9:  { kind: 'car', dir: -1, speed: 4.6, gap: 7.0, length: 1 },
  10: { kind: 'car', dir: +1, speed: 1.8, gap: 4.0, length: 2 }, // truck
  11: { kind: 'car', dir: -1, speed: 2.6, gap: 3.5, length: 1 },
};
const BASE_RIVER = {
  2:  { kind: 'log',    dir: +1, speed: 1.6, gap: 3.5, length: 3 },
  3:  { kind: 'turtle', dir: -1, speed: 2.0, gap: 3.0, length: 2 },
  4:  { kind: 'log',    dir: +1, speed: 2.6, gap: 4.5, length: 4 },
  5:  { kind: 'log',    dir: -1, speed: 2.0, gap: 3.5, length: 2 },
};

const LEVELS = [
  { name: ['Pond',     '池塘'], speedMul: 1.00, timeLimit: 60 },
  { name: ['Stream',   '小溪'], speedMul: 1.15, timeLimit: 55 },
  { name: ['River',    '大河'], speedMul: 1.30, timeLimit: 50 },
  { name: ['Rapids',   '急流'], speedMul: 1.50, timeLimit: 45 },
  { name: ['Torrent',  '湍流'], speedMul: 1.75, timeLimit: 40 },
  { name: ['Maelstrom','漩涡'], speedMul: 2.00, timeLimit: 36 },
  { name: ['Cataract', '飞瀑'], speedMul: 2.25, timeLimit: 33 },
  { name: ['Whitewater','激浪'], speedMul: 2.50, timeLimit: 30 },
  { name: ['Deluge',   '洪流'], speedMul: 2.80, timeLimit: 27 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- runtime state -----------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

function buildLane(row, base, mul, rng) {
  const speed = base.speed * mul * base.dir;
  const stride = base.length + base.gap;
  const offset = rng() * stride;
  // Fill the row with evenly spaced entities. Off-screen entities wrap
  // around mod (COLS + stride).
  const entities = [];
  for (let x = -stride + offset; x < COLS + stride; x += stride) {
    entities.push({ x, len: base.length });
  }
  return { row, kind: base.kind, dir: base.dir, speed, stride, length: base.length, entities };
}

function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const rng = seededRandom(levelIndex * 17 + 11);
  const lanes = [];
  for (const r of ROAD_ROWS)  lanes.push(buildLane(r, BASE_ROAD[r],  lv.speedMul, rng));
  for (const r of RIVER_ROWS) lanes.push(buildLane(r, BASE_RIVER[r], lv.speedMul, rng));
  return {
    levelIndex, lv,
    lanes,
    frog: { col: 4, row: ROW_START, x: 4, y: ROW_START, alive: true, hop: 0, face: 0 },
    pads: new Array(GOAL_COUNT).fill(false),
    lives: 3,
    score: 0,
    timeLeft: lv.timeLimit,
    over: false, won: false,
    flash: 0,                 // brief red/blue overlay on death/score
    bestCol: 4,               // furthest column reached for bonus
    bestRow: ROW_START,
  };
}

// ---- input -------------------------------------------------------------
function hop(s, dx, dy) {
  if (s.over || !s.frog.alive) return false;
  const nx = s.frog.col + dx;
  const ny = s.frog.row + dy;
  if (nx < 0 || nx >= COLS || ny < 0 || ny >= ROWS) return false;
  s.frog.col = nx; s.frog.row = ny;
  s.frog.x = nx; s.frog.y = ny;
  s.frog.hop = 0.18;
  s.frog.face = dx < 0 ? 2 : dx > 0 ? 1 : dy < 0 ? 0 : 3;
  if (ny < s.bestRow) {
    s.score += 10 * (s.bestRow - ny);
    s.bestRow = ny;
  }
  return true;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.timeLeft -= dt;
  s.flash = Math.max(0, s.flash - dt);
  s.frog.hop = Math.max(0, s.frog.hop - dt);
  // Wrap lane entities across the board.
  for (const lane of s.lanes) {
    for (const ent of lane.entities) {
      ent.x += lane.speed * dt;
      const total = COLS + lane.stride;
      if (lane.speed > 0) {
        while (ent.x >= total - lane.stride) ent.x -= total;
      } else {
        while (ent.x <= -lane.stride) ent.x += total;
      }
    }
  }
  // River carries the frog if it's standing on a log/turtle.
  if (RIVER_ROWS.includes(s.frog.row)) {
    const lane = laneAt(s, s.frog.row);
    const carrier = riderEntity(lane, s.frog.col);
    if (carrier) {
      s.frog.x += lane.speed * dt;
      // Snap col when the float drifts past a cell midpoint.
      if (s.frog.hop <= 0) {
        const snap = Math.round(s.frog.x);
        if (snap !== s.frog.col) {
          s.frog.col = snap;
        }
      }
      if (s.frog.x < -0.4 || s.frog.x > COLS - 0.6) {
        die(s);
      }
    } else {
      die(s);
    }
  } else {
    s.frog.x = s.frog.col;
  }
  // Road collisions.
  if (ROAD_ROWS.includes(s.frog.row) && s.frog.alive) {
    const lane = laneAt(s, s.frog.row);
    if (carHits(lane, s.frog.col)) die(s);
  }
  // Goal pad.
  if (s.frog.row === ROW_GOAL && s.frog.alive) {
    const idx = GOAL_COLS.indexOf(s.frog.col);
    if (idx >= 0 && !s.pads[idx]) {
      s.pads[idx] = true;
      s.score += 100 + Math.max(0, Math.floor(s.timeLeft * 5));
      resetFrog(s);
      s.flash = 0.4;
      if (s.pads.every(Boolean)) { s.over = true; s.won = true; }
    } else {
      die(s);
    }
  }
  if (s.timeLeft <= 0 && !s.over) die(s);
}

function laneAt(s, row) { return s.lanes.find(l => l.row === row); }

// Is any car / log on this lane covering the cell at `col`?
function carHits(lane, col) {
  for (const ent of lane.entities) {
    if (col >= ent.x - 0.5 && col < ent.x + lane.length - 0.5) return true;
  }
  return false;
}
// Same check but returns the entity (so the frog can ride it).
function riderEntity(lane, col) {
  for (const ent of lane.entities) {
    if (col >= ent.x - 0.5 && col < ent.x + lane.length - 0.5) return ent;
  }
  return null;
}

function die(s) {
  if (!s.frog.alive) return;
  s.frog.alive = false;
  s.flash = 0.35;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; }
  // Bring the frog back after a short respawn beat (driven by the game loop).
  s.frog.respawn = 0.55;
}

function resetFrog(s) {
  s.frog.col = 4; s.frog.row = ROW_START;
  s.frog.x = 4; s.frog.y = ROW_START;
  s.frog.alive = true;
  s.frog.face = 0;
  s.frog.hop = 0;
  s.bestRow = ROW_START;
}

// Score = total + remaining-time bonus + lives bonus. Used at level-end.
function levelScore(s) {
  return s.score + Math.max(0, Math.floor(s.timeLeft) * 10) + Math.max(0, s.lives) * 50;
}
