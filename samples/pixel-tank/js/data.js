// Pixel Tank - top-down PvE shooter inspired by Famicom Battle City.
//
// The play area is a 16x16 grid of 16-px cells (256x256), sitting below the
// HUD. Tanks are 14x14 px; bullets are 4x4 px. Walls and tanks live on the
// same grid, with collision tested as cell-aligned AABBs. Player defends the
// eagle and destroys every enemy tank to clear the stage.

const VW = 360, VH = 480;

const PLAY_X = 52;
const PLAY_Y = 56;
const CELL   = 16;
const GRID_W = 16;
const GRID_H = 16;
const PLAY_W = CELL * GRID_W;     // 256
const PLAY_H = CELL * GRID_H;     // 256

const TANK_SIZE  = 14;
const TANK_SPEED = 50;            // px / s
const BULLET_SIZE = 4;
const BULLET_SPEED = 200;
const ENEMY_FIRE_CD = 1.8;
const ENEMY_DIR_CD  = 1.0;

// Wall tile codes
const EMPTY = 0;
const BRICK = 1;
const STEEL = 2;
const EAGLE = 3;
const EAGLE_DEAD = 4;

// Each level: 16-row strings of 16 chars each
//   '.' empty, 'B' brick, 'S' steel, 'E' eagle, 'P' player spawn,
//   '1' '2' '3' enemy spawn slots
const LEVELS = [
  { name: ['Outpost',  '前哨'], rows: [
    '................',
    '.1......2......3',
    '................',
    '.....BBBBBB.....',
    '.....B....B.....',
    '.....B....B.....',
    '......BBBB......',
    '................',
    '...SS......SS...',
    '...SS......SS...',
    '................',
    '.....BBBBBB.....',
    '......BBBB......',
    '......B..B......',
    '......BEEB......',
    '......BBBB......',
  ] },
  { name: ['Fort',    '堡垒'], rows: [
    '1......B......2.',
    '.......B........',
    '.......B........',
    '..BBB.....BBB...',
    '..B.B.....B.B...',
    '..BBB.....BBB...',
    '................',
    '...3....SSSSSS..',
    '...........B....',
    '...........B....',
    '..BBB......B....',
    '..B.B......B....',
    '..BBB......B....',
    '..........BBB...',
    '..........BEB...',
    '..........BBB...',
  ] },
  { name: ['Citadel', '要塞'], rows: [
    '1...S.....S....2',
    '....S.....S.....',
    '....S.....S.....',
    '................',
    '.BBB.BBBB.BBBB..',
    '.B....B.B....B..',
    '.BBB.BB.B.BBBB..',
    '......B.B.......',
    '......B.B.......',
    '...3.SBBBBS.....',
    '....SSB..BSS....',
    '......B..B......',
    '......B..B......',
    '......BBBB......',
    '......BEEB......',
    '......BBBB......',
  ] },
  { name: ['Bastion', '城堡'], rows: [
    '...1.....2..3...',
    '.SSSSSSSSSSSSSS.',
    '................',
    '.B.B.B.B.B.B.B..',
    '.B.B.B.B.B.B.B..',
    '................',
    '.SS.SS.SS.SS.SS.',
    '................',
    '....BBBBBBBB....',
    '....B......B....',
    '....B.SSSS.B....',
    '....B.S..S.B....',
    '....B.S..S.B....',
    '....B.SBBSB.....',
    '....BBSBBSB.....',
    '......SEESS.....',
  ] },
  { name: ['Coliseum','斗兽场'], rows: [
    '..1...2....3....',
    '...BBBB.BBBB....',
    '...B........B...',
    '...B..SSSS..B...',
    '...B..S..S..B...',
    '...B..S..S..B...',
    '...B..SSSS..B...',
    '...B........B...',
    '...B........B...',
    '...B..BBBB..B...',
    '...B..B..B..B...',
    '...B..B..B..B...',
    '...BBBBBBBB.....',
    '................',
    '......BEEB......',
    '......BBBB......',
  ] },
  { name: ['Stronghold','要冲'], rows: [
    '1..2....3......B',
    '...............B',
    '.BBBBBB.SSSSSS.B',
    '.B.....B.....BB.',
    '.B.BBB.B.BBB.B..',
    '.B.B.B.B.B.B.B..',
    '.B.BBB.B.BBB.B..',
    '.B...........B..',
    '.B.SS.....SS.B..',
    '.B.SS.....SS.B..',
    '.B...........B..',
    '.B.BBB.B.BBB.B..',
    '.B.B.B.B.B.B.B..',
    '.B.BBB.B.BBB.B..',
    '.B.....BEE...B..',
    '.BBBBBBBBBBBBB..',
  ] },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const grid = new Array(GRID_W * GRID_H).fill(EMPTY);
  let playerSpawn = { x: 7, y: 13 };
  const enemySpawns = [];
  let eagleAt = null;
  for (let y = 0; y < GRID_H; y++) {
    const row = cfg.rows[y];
    for (let x = 0; x < GRID_W; x++) {
      const ch = row[x];
      if (ch === 'B') grid[y * GRID_W + x] = BRICK;
      else if (ch === 'S') grid[y * GRID_W + x] = STEEL;
      else if (ch === 'E') { grid[y * GRID_W + x] = EAGLE; eagleAt = { x, y }; }
      else if (ch === 'P') playerSpawn = { x, y };
      else if (ch === '1' || ch === '2' || ch === '3') enemySpawns.push({ x, y });
    }
  }
  const player = {
    x: playerSpawn.x * CELL + 1, y: playerSpawn.y * CELL + 1,
    dir: 0,   // 0=up 1=right 2=down 3=left
    alive: true, bullet: null,
  };
  const enemies = enemySpawns.map((sp, i) => ({
    x: sp.x * CELL + 1, y: sp.y * CELL + 1,
    dir: 2,
    fireCd: 0.6 + i * 0.4,
    dirCd: 0.5 + i * 0.3,
    alive: true, bullet: null,
  }));
  return {
    levelIndex, cfg, grid,
    player, enemies,
    eagleAt, eagleAlive: true,
    lives: 3,
    enemiesLeft: enemies.filter(e => e.alive).length,
    inputs: { up: false, right: false, down: false, left: false, fire: false, lastFire: false },
    rng: seededRandom(cfg.rows.join('').length * 113 + levelIndex),
    over: false, won: false, started: false,
    tickCount: 0,
  };
}

function setInput(s, kind, on) {
  if (s.over) return;
  s.inputs[kind] = !!on;
  if (on) s.started = true;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.tickCount++;
  movePlayer(s, dt);
  for (const e of s.enemies) if (e.alive) moveEnemy(s, e, dt);
  updateBullets(s, dt);
}

// ---- movement ---------------------------------------------------------
function movePlayer(s, dt) {
  const p = s.player;
  if (!p.alive) return;
  let dx = 0, dy = 0;
  if (s.inputs.up)    { dy = -1; p.dir = 0; }
  else if (s.inputs.right) { dx = 1; p.dir = 1; }
  else if (s.inputs.down)  { dy = 1; p.dir = 2; }
  else if (s.inputs.left)  { dx = -1; p.dir = 3; }
  if (dx || dy) tryMove(s, p, dx * TANK_SPEED * dt, dy * TANK_SPEED * dt);
  // Fire on rising edge of `fire`.
  if (s.inputs.fire && !s.inputs.lastFire) {
    if (!p.bullet) p.bullet = makeBullet(p);
  }
  s.inputs.lastFire = s.inputs.fire;
}

function moveEnemy(s, e, dt) {
  e.dirCd -= dt;
  if (e.dirCd <= 0) {
    e.dir = (s.rng() * 4) | 0;
    e.dirCd = ENEMY_DIR_CD * (0.6 + s.rng() * 0.8);
  }
  const [vx, vy] = DIR_VEC[e.dir];
  const blocked = !tryMove(s, e, vx * TANK_SPEED * 0.65 * dt, vy * TANK_SPEED * 0.65 * dt);
  if (blocked) e.dirCd = 0.05;          // pick a new direction soon
  e.fireCd -= dt;
  if (e.fireCd <= 0) {
    if (!e.bullet) e.bullet = makeBullet(e);
    e.fireCd = ENEMY_FIRE_CD * (0.7 + s.rng() * 0.6);
  }
}

const DIR_VEC = [[0, -1], [1, 0], [0, 1], [-1, 0]];

// Try to move tank by (dx, dy) px. Returns true on success.
function tryMove(s, tank, dx, dy) {
  // Move only along the dominant axis (no diagonal). And align to perpendicular axis to ease cornering.
  if (Math.abs(dx) > Math.abs(dy)) {
    dy = 0;
    // Snap y to nearest cell
    tank.y = Math.round(tank.y / CELL) * CELL + 1;
  } else {
    dx = 0;
    tank.x = Math.round(tank.x / CELL) * CELL + 1;
  }
  const nx = tank.x + dx;
  const ny = tank.y + dy;
  if (!tankFits(s, nx, ny, tank)) return false;
  tank.x = nx; tank.y = ny;
  return true;
}

function tankFits(s, x, y, ignoreTank) {
  if (x < 0 || y < 0 || x + TANK_SIZE > PLAY_W || y + TANK_SIZE > PLAY_H) return false;
  // Check wall cells overlapped by the AABB.
  const x0 = (x / CELL) | 0, x1 = ((x + TANK_SIZE - 1) / CELL) | 0;
  const y0 = (y / CELL) | 0, y1 = ((y + TANK_SIZE - 1) / CELL) | 0;
  for (let cy = y0; cy <= y1; cy++) for (let cx = x0; cx <= x1; cx++) {
    const tile = s.grid[cy * GRID_W + cx];
    if (tile === BRICK || tile === STEEL || tile === EAGLE) return false;
  }
  // Tank-vs-tank.
  const others = [s.player, ...s.enemies].filter(t => t.alive && t !== ignoreTank);
  for (const o of others) {
    if (x + TANK_SIZE <= o.x || o.x + TANK_SIZE <= x) continue;
    if (y + TANK_SIZE <= o.y || o.y + TANK_SIZE <= y) continue;
    return false;
  }
  return true;
}

// ---- bullets ----------------------------------------------------------
function makeBullet(tank) {
  const cx = tank.x + TANK_SIZE / 2, cy = tank.y + TANK_SIZE / 2;
  const [dx, dy] = DIR_VEC[tank.dir];
  return {
    x: cx - BULLET_SIZE / 2 + dx * (TANK_SIZE / 2),
    y: cy - BULLET_SIZE / 2 + dy * (TANK_SIZE / 2),
    vx: dx, vy: dy,
    owner: tank,
  };
}

function updateBullets(s, dt) {
  const bullets = [];
  if (s.player.bullet) bullets.push({ b: s.player.bullet, tank: s.player });
  for (const e of s.enemies) if (e.bullet) bullets.push({ b: e.bullet, tank: e });
  for (const { b, tank } of bullets) {
    b.x += b.vx * BULLET_SPEED * dt;
    b.y += b.vy * BULLET_SPEED * dt;
    if (b.x < 0 || b.y < 0 || b.x > PLAY_W || b.y > PLAY_H) {
      tank.bullet = null;
      continue;
    }
    // Wall hit?
    const cx = ((b.x + BULLET_SIZE / 2) / CELL) | 0;
    const cy = ((b.y + BULLET_SIZE / 2) / CELL) | 0;
    if (cx < 0 || cy < 0 || cx >= GRID_W || cy >= GRID_H) { tank.bullet = null; continue; }
    const tile = s.grid[cy * GRID_W + cx];
    if (tile === BRICK) {
      s.grid[cy * GRID_W + cx] = EMPTY;
      tank.bullet = null;
      continue;
    }
    if (tile === STEEL) {
      tank.bullet = null;
      continue;
    }
    if (tile === EAGLE) {
      s.grid[cy * GRID_W + cx] = EAGLE_DEAD;
      s.eagleAlive = false;
      s.over = true; s.won = false;
      tank.bullet = null;
      continue;
    }
    // Tank hit?
    const targets = (tank === s.player) ? s.enemies : [s.player, ...s.enemies.filter(e => e !== tank)];
    for (const t of targets) {
      if (!t.alive) continue;
      if (b.x + BULLET_SIZE <= t.x || t.x + TANK_SIZE <= b.x) continue;
      if (b.y + BULLET_SIZE <= t.y || t.y + TANK_SIZE <= b.y) continue;
      // Hit.
      t.alive = false;
      t.bullet = null;       // their bullet drops too
      tank.bullet = null;
      if (t === s.player) onPlayerHit(s);
      else s.enemiesLeft = s.enemies.filter(e => e.alive).length;
      break;
    }
  }
  // Win check.
  if (!s.over && s.enemiesLeft === 0) {
    s.over = true; s.won = true;
  }
}

function onPlayerHit(s) {
  s.lives--;
  if (s.lives <= 0) { s.over = true; s.won = false; return; }
  // Respawn at original spawn (bottom centre).
  const row = s.cfg.rows;
  let px = 7, py = 13;
  for (let y = 0; y < GRID_H; y++) {
    for (let x = 0; x < GRID_W; x++) if (row[y][x] === 'P') { px = x; py = y; }
  }
  s.player.alive = true;
  s.player.bullet = null;
  s.player.dir = 0;
  s.player.x = px * CELL + 1;
  s.player.y = py * CELL + 1;
}
