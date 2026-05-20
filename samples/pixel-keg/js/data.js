// Pixel Keg - a Bomberman-style dungeon: drop powder kegs, blast bricks, find
// the hidden stairs. Indestructible walls form the usual lattice; the rest is
// floor that may be scattered with destructible bricks.

const VW = 360, VH = 480;
const N = 11;                  // grid is N x N
const NN = N * N;

// tile types
const FLOOR = 0, WALL = 1, BRICK = 2;
// directions (up, right, down, left)
const DIRS = [{ dr: -1, dc: 0 }, { dr: 0, dc: 1 }, { dr: 1, dc: 0 }, { dr: 0, dc: -1 }];

const FUSE = 2.0;
const FLAME_LIFE = 0.42;
const ENEMY_STEP = 0.55;
const RESPAWN_DELAY = 0.6;

const LEVELS = [
  { name: ['Crypt', '地穴'],     seed: 13,  bricks:  8, enemies: 1, range: 2 },
  { name: ['Cellar', '酒窖'],    seed: 41,  bricks: 12, enemies: 2, range: 2 },
  { name: ['Catacomb', '墓道'],  seed: 86,  bricks: 16, enemies: 2, range: 2 },
  { name: ['Vault', '密室'],     seed: 152, bricks: 20, enemies: 3, range: 3 },
  { name: ['Citadel', '城堡'],   seed: 235, bricks: 24, enemies: 3, range: 3 },
  { name: ['Inferno', '炼狱'],   seed: 337, bricks: 28, enemies: 4, range: 3 },
  { name: ['Abyss',   '深渊'],   seed: 458, bricks: 32, enemies: 5, range: 3 },
  { name: ['Hellmaw', '魔咽'],   seed: 591, bricks: 36, enemies: 6, range: 4 },
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

const idx = (r, c) => r * N + c;
const rowOf = i => (i / N) | 0;
const colOf = i => i % N;

// Build a level: grid + player, enemies, hidden exit.
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(cfg.seed);
  const grid = new Int8Array(NN);
  for (let r = 0; r < N; r++) for (let c = 0; c < N; c++) {
    let t = FLOOR;
    if (r === 0 || c === 0 || r === N - 1 || c === N - 1) t = WALL;
    else if (r % 2 === 0 && c % 2 === 0) t = WALL;
    grid[idx(r, c)] = t;
  }
  // safety zone: top-left corner stays open
  const safe = new Set([idx(1, 1), idx(1, 2), idx(2, 1), idx(1, 3), idx(3, 1)]);
  // collect floor cells, drop bricks
  const floors = [];
  for (let i = 0; i < NN; i++) if (grid[i] === FLOOR && !safe.has(i)) floors.push(i);
  shuffle(floors, rng);
  const bricks = Math.min(cfg.bricks, floors.length);
  for (let k = 0; k < bricks; k++) grid[floors[k]] = BRICK;
  // exit hidden under a random brick
  const brickList = [];
  for (let i = 0; i < NN; i++) if (grid[i] === BRICK) brickList.push(i);
  const exit = brickList.length ? brickList[(rng() * brickList.length) | 0] : floors[0];
  // enemy spawns: floor cells far from player
  const player = idx(1, 1);
  const farFloors = [];
  for (let i = 0; i < NN; i++) {
    if (grid[i] !== FLOOR || i === player) continue;
    const r = rowOf(i), c = colOf(i);
    if (r + c >= 11) farFloors.push(i);
  }
  shuffle(farFloors, rng);
  const enemies = [];
  for (let k = 0; k < cfg.enemies && k < farFloors.length; k++) {
    enemies.push({ idx: farFloors[k], stepT: 0.3 + rng() * 0.5, alive: true });
  }
  return {
    levelIndex, cfg, grid, exit, exitRevealed: false,
    player: { idx: player, alive: true, respawnT: 0, startIdx: player },
    enemies, bombs: [], flames: [],
    lives: 3, won: false, over: false,
  };
}

function shuffle(a, rng) {
  for (let k = a.length - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [a[k], a[j]] = [a[j], a[k]];
  }
}

// ---- queries -------------------------------------------------------------
function bombAt(s, i) { return s.bombs.find(b => b.idx === i); }
function enemyAt(s, i) { return s.enemies.find(e => e.alive && e.idx === i); }
function flameAt(s, i) { return s.flames.some(f => f.idx === i); }
function blockedForMove(s, i) {
  const t = s.grid[i];
  return t !== FLOOR || !!bombAt(s, i);
}

// ---- input actions -------------------------------------------------------
function movePlayer(s, dir) {
  if (s.over || !s.player.alive) return false;
  const r = rowOf(s.player.idx), c = colOf(s.player.idx);
  const d = DIRS[dir];
  const nr = r + d.dr, nc = c + d.dc;
  if (nr < 0 || nc < 0 || nr >= N || nc >= N) return false;
  const ni = idx(nr, nc);
  if (blockedForMove(s, ni)) return false;
  s.player.idx = ni;
  checkPlayerHazards(s);
  if (!s.over && s.exitRevealed && s.player.idx === s.exit) winLevel(s);
  return true;
}

function placeBomb(s) {
  if (s.over || !s.player.alive) return false;
  if (bombAt(s, s.player.idx)) return false;
  if (s.bombs.length >= 1) return false;     // capacity 1
  s.bombs.push({ idx: s.player.idx, fuse: FUSE, range: s.cfg.range });
  return true;
}

// ---- world tick ----------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  // respawn timer
  if (!s.player.alive) {
    s.player.respawnT -= dt;
    if (s.player.respawnT <= 0) {
      s.player.alive = true;
      s.player.idx = s.player.startIdx;
    }
  }
  // bombs
  for (const b of s.bombs) b.fuse -= dt;
  const toExplode = s.bombs.filter(b => b.fuse <= 0);
  s.bombs = s.bombs.filter(b => b.fuse > 0);
  for (const b of toExplode) explode(s, b);
  // flames
  for (const f of s.flames) f.life -= dt;
  s.flames = s.flames.filter(f => f.life > 0);
  // enemies
  for (const e of s.enemies) {
    if (!e.alive) continue;
    e.stepT -= dt;
    if (e.stepT <= 0) {
      stepEnemy(s, e);
      e.stepT = ENEMY_STEP;
    }
  }
  checkPlayerHazards(s);
}

function explode(s, bomb) {
  const cells = [bomb.idx];
  for (const d of DIRS) {
    let r = rowOf(bomb.idx), c = colOf(bomb.idx);
    for (let k = 1; k <= bomb.range; k++) {
      r += d.dr; c += d.dc;
      if (r < 0 || c < 0 || r >= N || c >= N) break;
      const i = idx(r, c);
      const t = s.grid[i];
      if (t === WALL) break;
      cells.push(i);
      if (t === BRICK) { s.grid[i] = FLOOR; if (i === s.exit) s.exitRevealed = true; break; }
    }
  }
  for (const i of cells) s.flames.push({ idx: i, life: FLAME_LIFE });
  // chain reaction: any bomb caught by a flame goes off too
  const caught = s.bombs.filter(b => cells.includes(b.idx));
  s.bombs = s.bombs.filter(b => !cells.includes(b.idx));
  for (const b of caught) explode(s, b);
  // enemies caught die
  for (const e of s.enemies) if (e.alive && cells.includes(e.idx)) e.alive = false;
}

function stepEnemy(s, e) {
  const order = [0, 1, 2, 3];
  shuffleU(order, e);
  const r = rowOf(e.idx), c = colOf(e.idx);
  for (const di of order) {
    const d = DIRS[di];
    const nr = r + d.dr, nc = c + d.dc;
    if (nr < 0 || nc < 0 || nr >= N || nc >= N) continue;
    const ni = idx(nr, nc);
    if (s.grid[ni] !== FLOOR || bombAt(s, ni)) continue;
    if (enemyAt(s, ni)) continue;
    e.idx = ni;
    return;
  }
}
// per-enemy non-rng-shuffle (simple cycle for determinism)
function shuffleU(arr, e) {
  e._tick = ((e._tick || 0) + 1) & 3;
  for (let k = 0; k < e._tick; k++) arr.push(arr.shift());
}

function checkPlayerHazards(s) {
  if (s.over || !s.player.alive) return;
  if (flameAt(s, s.player.idx) || enemyAt(s, s.player.idx)) killPlayer(s);
}

function killPlayer(s) {
  s.player.alive = false;
  s.player.respawnT = RESPAWN_DELAY;
  s.lives--;
  if (s.lives <= 0) { s.over = true; s.won = false; }
}

function winLevel(s) {
  s.over = true; s.won = true;
}
