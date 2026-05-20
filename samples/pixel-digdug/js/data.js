// Pixel DigDug - a tribute to the Atari classic. The world is a 16x20
// dirt-filled grid; the player digs tunnels by walking through dirt,
// and pumps enemies along an unobstructed in-line view until they pop.
//
// Tiles per cell:
//   0 = dirt          (impassable until dug)
//   1 = sky / tunnel  (walkable)
//   2 = rock          (falls when the cell directly below it is empty)

const VW = 360, VH = 480;
const COLS = 16;
const ROWS = 20;
const CELL = 20;
const BOARD_OX = ((VW - COLS * CELL) / 2) | 0;       // 20
const BOARD_OY = 32;
const SKY_ROWS = 2;                                  // top rows start as sky

const PLAYER_SPEED = 3.6;       // cells/sec
const ENEMY_SPEED  = 2.4;
const PUMP_TIME    = 0.55;      // seconds for one pump increment
const POP_PUMPS    = 3;          // pumps to pop an enemy

// ---- levels ------------------------------------------------------------
const LEVELS = [
  { name: ['Topsoil',   '表土'], enemies: 2, rocks: 0, enemySpeed: 1.8 },
  { name: ['Garden',    '园圃'], enemies: 3, rocks: 1, enemySpeed: 2.1 },
  { name: ['Bedrock',   '基岩'], enemies: 3, rocks: 2, enemySpeed: 2.4 },
  { name: ['Cavern',    '洞窟'], enemies: 4, rocks: 3, enemySpeed: 2.7 },
  { name: ['Abyss',     '深渊'], enemies: 5, rocks: 4, enemySpeed: 3.0 },
  { name: ['Mantle',    '地幔'], enemies: 6, rocks: 5, enemySpeed: 3.3 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}
function idx(c, r) { return r * COLS + c; }
function inBounds(c, r) { return c >= 0 && r >= 0 && c < COLS && r < ROWS; }

// Carve a small starting tunnel from the player's spawn down into the dirt
// so the player can start moving immediately without "tunnelling out of"
// the spawn point.
function buildBoard(rng, cfg) {
  const tiles = new Uint8Array(COLS * ROWS);
  for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
    tiles[idx(c, r)] = r < SKY_ROWS ? 1 : 0;
  }
  // Drop rocks at scattered dirt cells (never in the sky rows + leave a
  // 2-cell buffer around the player spawn so a rock doesn't fall on them
  // turn one).
  const spawnC = (COLS / 2) | 0, spawnR = SKY_ROWS + 1;
  // Carve a 1-cell wide vertical spawn shaft from sky to spawn.
  for (let r = SKY_ROWS; r <= spawnR; r++) tiles[idx(spawnC, r)] = 1;
  const rocks = [];
  for (let placed = 0; placed < cfg.rocks; ) {
    const c = (rng() * COLS) | 0;
    const r = SKY_ROWS + 2 + ((rng() * (ROWS - SKY_ROWS - 4)) | 0);
    if (Math.abs(c - spawnC) <= 1 && Math.abs(r - spawnR) <= 2) continue;
    if (tiles[idx(c, r)] !== 0) continue;
    tiles[idx(c, r)] = 2;
    rocks.push({ c, r, vy: 0, falling: false, dead: false });
    placed++;
  }
  return { tiles, rocks, spawnC, spawnR };
}

// ---- enemy + pump state ------------------------------------------------
function spawnEnemy(s, c, r, color) {
  return {
    c, r, x: c, y: r,                 // float pos in cell units
    dir: { x: 0, y: 1 },
    color,
    pumped: 0,                        // 0 .. POP_PUMPS
    pumpDecay: 0,
    alive: true,
    moveAcc: 0,
  };
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(levelIndex * 53 + 7);
  const b = buildBoard(rng, cfg);
  // Place enemies in deep dirt pockets — pre-carve a one-cell room each.
  const enemies = [];
  const ePalette = ['#ff5a5a', '#5fc06e', '#bda6ff', '#ffd34a', '#5fc0ff', '#ff8fd0'];
  let placed = 0, tries = 0;
  while (placed < cfg.enemies && tries++ < 200) {
    const c = 1 + ((rng() * (COLS - 2)) | 0);
    const r = SKY_ROWS + 3 + ((rng() * (ROWS - SKY_ROWS - 4)) | 0);
    if (Math.abs(c - b.spawnC) <= 2 && r <= b.spawnR + 2) continue;
    if (b.tiles[idx(c, r)] !== 0) continue;
    b.tiles[idx(c, r)] = 1;          // pre-carve a 1-cell pocket
    enemies.push(spawnEnemy({}, c, r, ePalette[placed % ePalette.length]));
    placed++;
  }
  return {
    levelIndex, cfg,
    tiles: b.tiles,
    rocks: b.rocks,
    player: {
      c: b.spawnC, r: b.spawnR, x: b.spawnC, y: b.spawnR,
      dir: { x: 0, y: 1 }, face: 'down',
      moveAcc: 0,
      alive: true, hitFlash: 0, respawn: 0,
    },
    enemies,
    pumpTargetIdx: -1,   // index into enemies for the currently pumped one
    pumpHold: 0,         // accumulator toward the next pump increment
    pumping: false,
    inputDir: null,      // most recent input direction
    inputPump: false,
    score: 0,
    lives: 2,
    over: false, won: false,
    flash: 0,
  };
}

// ---- tile query helpers ------------------------------------------------
function tileAt(s, c, r) {
  if (!inBounds(c, r)) return 0;     // OOB treated as dirt (no walking off)
  return s.tiles[idx(c, r)];
}
function isOpen(s, c, r) {
  const t = tileAt(s, c, r);
  return t === 1;                     // walkable iff sky / tunnel
}

// ---- input -------------------------------------------------------------
function setMove(s, dir) {
  if (s.over || !s.player.alive) return;
  s.inputDir = dir;     // 'left' | 'right' | 'up' | 'down' | null
}
function setPump(s, on) { s.inputPump = on; }

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  if (!s.player.alive) {
    s.player.respawn -= dt;
    if (s.player.respawn <= 0) resetPlayer(s);
    return;
  }
  movePlayer(s, dt);
  handlePump(s, dt);
  moveEnemies(s, dt);
  updateRocks(s, dt);
  checkCollisions(s);
}

function movePlayer(s, dt) {
  const p = s.player;
  const dir = s.inputDir;
  if (!dir) return;
  // Translate string dir -> step vector
  const v = dir === 'left'  ? { x: -1, y:  0 } :
            dir === 'right' ? { x:  1, y:  0 } :
            dir === 'up'    ? { x:  0, y: -1 } :
            dir === 'down'  ? { x:  0, y:  1 } : { x: 0, y: 0 };
  p.dir = v; p.face = dir;
  p.moveAcc += dt * PLAYER_SPEED;
  while (p.moveAcc >= 1) {
    p.moveAcc -= 1;
    const nc = p.c + v.x, nr = p.r + v.y;
    if (!inBounds(nc, nr)) return;
    const t = tileAt(s, nc, nr);
    if (t === 2) return;            // rocks block walking
    // Walking into dirt carves a tunnel; sky cells are free.
    if (t === 0) { s.tiles[idx(nc, nr)] = 1; s.score += 1; }
    p.c = nc; p.r = nr; p.x = nc; p.y = nr;
  }
}

// Pump rules: the player must be facing an enemy along a line of cells
// that are ALL walkable (tunnel) between them. A single pump increment
// accrues every PUMP_TIME seconds the pump button is held.
function handlePump(s, dt) {
  if (!s.inputPump) {
    // Release: target deflates a little each frame.
    if (s.pumpTargetIdx >= 0) {
      const e = s.enemies[s.pumpTargetIdx];
      if (e && e.alive) {
        e.pumpDecay += dt;
        if (e.pumpDecay > 0.6 && e.pumped > 0) {
          e.pumped--;
          e.pumpDecay = 0;
        }
      }
      s.pumpTargetIdx = -1;
      s.pumpHold = 0;
    }
    s.pumping = false;
    return;
  }
  const tgtIdx = findPumpTarget(s);
  if (tgtIdx < 0) { s.pumpTargetIdx = -1; s.pumpHold = 0; s.pumping = false; return; }
  if (tgtIdx !== s.pumpTargetIdx) { s.pumpTargetIdx = tgtIdx; s.pumpHold = 0; }
  s.pumping = true;
  s.pumpHold += dt;
  if (s.pumpHold >= PUMP_TIME) {
    s.pumpHold -= PUMP_TIME;
    const e = s.enemies[tgtIdx];
    e.pumped++;
    e.pumpDecay = 0;
    if (e.pumped >= POP_PUMPS) {
      e.alive = false;
      s.score += 200;
      s.flash = 0.3;
      s.pumpTargetIdx = -1;
    }
  }
}

function findPumpTarget(s) {
  // Cast a ray from the player along its facing direction up to 6 cells;
  // succeed if the first cell with an enemy is reached without hitting
  // a non-walkable tile.
  const p = s.player;
  const v = p.dir;
  if (!v || (v.x === 0 && v.y === 0)) return -1;
  let cx = p.c, cy = p.r;
  for (let k = 0; k < 6; k++) {
    cx += v.x; cy += v.y;
    if (!inBounds(cx, cy)) return -1;
    if (!isOpen(s, cx, cy)) return -1;
    for (let i = 0; i < s.enemies.length; i++) {
      const e = s.enemies[i];
      if (!e.alive) continue;
      if (e.c === cx && e.r === cy) return i;
    }
  }
  return -1;
}

// Enemies walk through tunnels toward the player by greedy gradient.
// While pumped > 0 the enemy is stunned (no movement) so the player can
// finish the kill.
function moveEnemies(s, dt) {
  const sp = s.cfg.enemySpeed;
  for (const e of s.enemies) {
    if (!e.alive) continue;
    if (e.pumped > 0) continue;
    e.moveAcc += dt * sp;
    while (e.moveAcc >= 1) {
      e.moveAcc -= 1;
      const choice = pickEnemyStep(s, e);
      if (!choice) break;
      e.c += choice.x; e.r += choice.y;
      e.x = e.c; e.y = e.r;
      e.dir = choice;
    }
  }
}

function pickEnemyStep(s, e) {
  // Prefer the cardinal direction whose adjacent open cell minimises
  // distance to the player. Never reverse direction unless dead-end.
  const dirs = [{x:1,y:0},{x:-1,y:0},{x:0,y:1},{x:0,y:-1}];
  const rev = { x: -e.dir.x, y: -e.dir.y };
  const opts = [];
  for (const d of dirs) {
    const nc = e.c + d.x, nr = e.r + d.y;
    if (!isOpen(s, nc, nr)) continue;
    const dist = Math.abs(nc - s.player.c) + Math.abs(nr - s.player.r);
    opts.push({ d, dist, isRev: d.x === rev.x && d.y === rev.y });
  }
  if (!opts.length) return null;
  const fwd = opts.filter(o => !o.isRev);
  const pool = fwd.length ? fwd : opts;
  pool.sort((a, b) => a.dist - b.dist);
  return pool[0].d;
}

// Rocks fall when the cell directly below becomes empty.
function updateRocks(s, dt) {
  for (const rk of s.rocks) {
    if (rk.dead) continue;
    if (!rk.falling) {
      const below = tileAt(s, rk.c, rk.r + 1);
      if (below === 1) {
        rk.falling = true;
        // Cell now empty so the rock can fall.
        s.tiles[idx(rk.c, rk.r)] = 1;
      }
    } else {
      rk.vy += 8 * dt;
      rk.y = (rk.y == null ? rk.r : rk.y) + rk.vy * dt;
      const cellR = Math.floor(rk.y);
      if (cellR > rk.r) {
        // Crossed a cell boundary.
        rk.r = cellR;
        // Any enemy at (rk.c, rk.r)? squash it.
        for (const e of s.enemies) {
          if (!e.alive) continue;
          if (e.c === rk.c && e.r === rk.r) { e.alive = false; s.score += 400; s.flash = 0.3; }
        }
        // Hit the player?
        if (s.player.c === rk.c && s.player.r === rk.r) { die(s); }
      }
      const nextR = Math.floor(rk.y) + 1;
      if (!inBounds(rk.c, nextR) || tileAt(s, rk.c, nextR) !== 1) {
        // Rock stops here.
        rk.falling = false;
        rk.vy = 0;
        // Final smash: occupies its cell as a small heap; remove it
        // entirely after a short delay (just mark dead).
        rk.dead = true;
      }
    }
  }
}

function checkCollisions(s) {
  const p = s.player;
  if (!p.alive) return;
  for (const e of s.enemies) {
    if (!e.alive) continue;
    if (e.c === p.c && e.r === p.r) { die(s); return; }
  }
  // Wave clear when no enemy remains alive.
  if (s.enemies.every(e => !e.alive)) {
    s.over = true; s.won = true;
    s.score += 500;
    s.flash = 0.5;
  }
}

function die(s) {
  const p = s.player;
  if (!p.alive) return;
  p.alive = false;
  p.hitFlash = 0.6;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  p.respawn = 0.7;
  // Reset any pump in progress.
  if (s.pumpTargetIdx >= 0) {
    const e = s.enemies[s.pumpTargetIdx];
    if (e) e.pumped = 0;
    s.pumpTargetIdx = -1;
  }
}

function resetPlayer(s) {
  s.player.c = (COLS / 2) | 0;
  s.player.r = SKY_ROWS + 1;
  s.player.x = s.player.c; s.player.y = s.player.r;
  s.player.alive = true;
  s.player.dir = { x: 0, y: 1 };
  s.player.face = 'down';
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 100;
}
