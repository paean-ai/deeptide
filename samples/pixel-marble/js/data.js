// Pixel Marble - top-down tilt-to-roll. Drag from anywhere indicates the
// tilt direction; the marble accelerates that way. Avoid the holes, reach
// the green goal pad. Walls are cell-aligned; the marble is a 6-px circle.

const VW = 360, VH = 480;

const PLAY_X  = 20;
const PLAY_Y  = 60;
const CELL    = 20;
const GRID_W  = 16;
const GRID_H  = 16;
const PLAY_W  = CELL * GRID_W;     // 320
const PLAY_H  = CELL * GRID_H;     // 320

const BALL_R   = 6;
const TILT_ACC = 760;              // px / s^2 at full tilt
const MAX_VEL  = 320;
const FRICTION = 0.7;              // per second; vel *= FRICTION ^ dt
const HOLE_R   = 7;                // any centre-distance below this == fall
const WALL_BOUNCE = 0.35;          // energy retained on a wall bounce

// '.' empty  '#' wall  'O' hole  'S' marble spawn  'G' goal pad
const LEVELS = [
  { name: ['Foyer',  '前厅'], rows: [
    '################',
    '#S.............#',
    '#..............#',
    '#......##......#',
    '#......##......#',
    '#..............#',
    '#......OO......#',
    '#..............#',
    '#......OO......#',
    '#..............#',
    '#......##......#',
    '#......##......#',
    '#..............#',
    '#.............G#',
    '#..............#',
    '################',
  ] },
  { name: ['Atrium', '中庭'], rows: [
    '################',
    '#S....##.......#',
    '#.....##.......#',
    '#...........O..#',
    '#..#####.......#',
    '#......#...##..#',
    '#.O....#...##..#',
    '#......#.......#',
    '#......#####...#',
    '#..#...........#',
    '#..#......O....#',
    '#..#####.......#',
    '#..........##..#',
    '#..O.......##.G#',
    '#..............#',
    '################',
  ] },
  { name: ['Maze', '迷宫'], rows: [
    '################',
    '#S....#......G.#',
    '#.....#........#',
    '#.###.#.######.#',
    '#.#...#......#.#',
    '#.#.###.###..#.#',
    '#.#.O.#.#.O..#.#',
    '#.#.#.#.#.####.#',
    '#.#.#.#.#......#',
    '#.#.#.#.#####.##',
    '#...#.#.....#..#',
    '###.#.#####.#..#',
    '#...#.O.O.#.#..#',
    '#.###.#####.#..#',
    '#.....O........#',
    '################',
  ] },
  { name: ['Trap', '陷阱'], rows: [
    '################',
    '#S.O....O....O.#',
    '#..............#',
    '#.O..######..O.#',
    '#....#....#....#',
    '#....#.OO.#....#',
    '#.O..#.##.#..O.#',
    '#....#.##.#....#',
    '#.O..#.OO.#..O.#',
    '#....#....#....#',
    '#....######....#',
    '#.O..........O.#',
    '#..............#',
    '#.O.O.O..O.O.O.#',
    '#.............G#',
    '################',
  ] },
  { name: ['Spiral', '螺旋'], rows: [
    '################',
    '#S.............#',
    '#.############.#',
    '#.#..........#.#',
    '#.#.########.#.#',
    '#.#.#......#.#.#',
    '#.#.#.####.#.#.#',
    '#.#.#.#G.#.#.#.#',
    '#.#.#.####.#.#.#',
    '#.#.#......#.#.#',
    '#.#.########.#.#',
    '#.#..........#.#',
    '#.############.#',
    '#..O........O..#',
    '#..............#',
    '################',
  ] },
  { name: ['Gauntlet', '试炼'], rows: [
    '################',
    '#S.O.O.O.O.O.O.#',
    '#..............#',
    '#.############.#',
    '#.O.O.O.O.O.O.##',
    '#..............#',
    '#.############.#',
    '#.O..O...O..O..#',
    '#....O.OO.O....#',
    '#..............#',
    '#.O.O.O.O.O.O..#',
    '#.O....OO....O.#',
    '#..............#',
    '#.############.#',
    '#............O.#',
    '##############G#',
  ] },
];
const LEVEL_COUNT = LEVELS.length;

const EMPTY = 0, WALL = 1, HOLE = 2, GOAL = 3;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const grid = new Array(GRID_W * GRID_H).fill(EMPTY);
  let spawn = { x: 1, y: 1 };
  let goalAt = null;
  for (let y = 0; y < GRID_H; y++) for (let x = 0; x < GRID_W; x++) {
    const ch = cfg.rows[y][x];
    const idx = y * GRID_W + x;
    if      (ch === '#') grid[idx] = WALL;
    else if (ch === 'O') grid[idx] = HOLE;
    else if (ch === 'G') { grid[idx] = GOAL; goalAt = { x, y }; }
    else if (ch === 'S') spawn = { x, y };
  }
  return {
    levelIndex, cfg, grid, goalAt,
    ball: {
      x: spawn.x * CELL + CELL / 2,
      y: spawn.y * CELL + CELL / 2,
      vx: 0, vy: 0, alive: true,
    },
    tilt: { x: 0, y: 0 },
    started: false,
    over: false, won: false,
    elapsed: 0,
  };
}

// `tiltX`, `tiltY` in [-1, 1] (normalised drag vector).
function setTilt(s, tiltX, tiltY) {
  if (s.over) return;
  s.tilt.x = Math.max(-1, Math.min(1, tiltX));
  s.tilt.y = Math.max(-1, Math.min(1, tiltY));
  if (s.tilt.x || s.tilt.y) s.started = true;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.elapsed += dt;
  // 240Hz substep for stable wall collisions on fast rolls.
  const sub = 1 / 240;
  let remaining = dt;
  while (remaining > 0 && !s.over) {
    const step = Math.min(sub, remaining);
    substep(s, step);
    remaining -= step;
  }
}

function substep(s, dt) {
  const b = s.ball;
  if (!b.alive) return;
  // Acceleration from tilt + friction.
  b.vx += s.tilt.x * TILT_ACC * dt;
  b.vy += s.tilt.y * TILT_ACC * dt;
  const fk = Math.pow(FRICTION, dt);
  b.vx *= fk; b.vy *= fk;
  const sp = Math.hypot(b.vx, b.vy);
  if (sp > MAX_VEL) { b.vx *= MAX_VEL / sp; b.vy *= MAX_VEL / sp; }
  // Move along x then y; resolve wall collisions per axis.
  b.x += b.vx * dt;
  resolveWalls(s, true);
  b.y += b.vy * dt;
  resolveWalls(s, false);
  // Hole / goal check at the ball centre.
  const cx = (b.x / CELL) | 0, cy = (b.y / CELL) | 0;
  if (cx < 0 || cy < 0 || cx >= GRID_W || cy >= GRID_H) { b.alive = false; s.over = true; return; }
  const tile = s.grid[cy * GRID_W + cx];
  if (tile === HOLE) {
    const hx = cx * CELL + CELL / 2, hy = cy * CELL + CELL / 2;
    if (Math.hypot(b.x - hx, b.y - hy) < HOLE_R) {
      b.alive = false;
      s.over = true; s.won = false;
      return;
    }
  } else if (tile === GOAL) {
    s.over = true; s.won = true;
    return;
  }
}

function resolveWalls(s, isX) {
  const b = s.ball;
  // Walls are cell-aligned; check the cells the ball overlaps.
  const x0 = ((b.x - BALL_R) / CELL) | 0;
  const x1 = ((b.x + BALL_R) / CELL) | 0;
  const y0 = ((b.y - BALL_R) / CELL) | 0;
  const y1 = ((b.y + BALL_R) / CELL) | 0;
  for (let cy = y0; cy <= y1; cy++) for (let cx = x0; cx <= x1; cx++) {
    if (cx < 0 || cy < 0 || cx >= GRID_W || cy >= GRID_H) continue;
    if (s.grid[cy * GRID_W + cx] !== WALL) continue;
    const wx0 = cx * CELL, wy0 = cy * CELL;
    const wx1 = wx0 + CELL, wy1 = wy0 + CELL;
    if (b.x + BALL_R <= wx0 || wx1 <= b.x - BALL_R) continue;
    if (b.y + BALL_R <= wy0 || wy1 <= b.y - BALL_R) continue;
    // Overlap: push along the resolved axis.
    if (isX) {
      if (b.vx > 0) { b.x = wx0 - BALL_R; b.vx = -b.vx * WALL_BOUNCE; }
      else          { b.x = wx1 + BALL_R; b.vx = -b.vx * WALL_BOUNCE; }
    } else {
      if (b.vy > 0) { b.y = wy0 - BALL_R; b.vy = -b.vy * WALL_BOUNCE; }
      else          { b.y = wy1 + BALL_R; b.vy = -b.vy * WALL_BOUNCE; }
    }
  }
}
