// Pixel Cycle - a Tron-style lightcycle duel. Each cycle leaves a trail; ride
// into ANY trail or wall and you're out. Best of 3 wins the match.

const VW = 360, VH = 480;
const GRID_W = 30, GRID_H = 34;
const CELL = 12;
const BOARD_X = (VW - GRID_W * CELL) / 2;
const BOARD_Y = 60;

// trail markers in the grid
const EMPTY = 0, PLAYER_TRAIL = 1, CPU_TRAIL = 2;
// direction vectors
const DIRS = [{ dx: 0, dy: -1 }, { dx: 1, dy: 0 }, { dx: 0, dy: 1 }, { dx: -1, dy: 0 }];

const ROUND_DELAY = 0.9;
const TICK_INTERVAL = 0.09;
const ROUNDS_TO_WIN = 2;
const MAX_ROUNDS = 7;       // ties (both crash same tick) score 0 — cap rounds

const LEVELS = [
  { name: ['Rookie', '新兵'],   seed: 13,  ai: 'easy' },
  { name: ['Cadet', '学员'],    seed: 47,  ai: 'easy' },
  { name: ['Sergeant', '中士'], seed: 96,  ai: 'medium' },
  { name: ['Captain', '上尉'],  seed: 162, ai: 'medium' },
  { name: ['Colonel', '上校'],  seed: 245, ai: 'hard' },
  { name: ['General', '将军'],  seed: 348, ai: 'hard' },
  { name: ['Marshal', '元帅'],  seed: 471, ai: 'elite' },
  { name: ['Overlord', '霸主'], seed: 612, ai: 'elite' },
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

const ix = (x, y) => y * GRID_W + x;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const s = {
    levelIndex, cfg,
    rng: seededRandom(cfg.seed),
    playerWins: 0, cpuWins: 0, roundsPlayed: 0,
    over: false, won: false,
  };
  startRound(s);
  return s;
}

function startRound(s) {
  s.grid = new Uint8Array(GRID_W * GRID_H);
  s.player = { x: 5,         y: (GRID_H >> 1), d: 1, alive: true };  // facing right
  s.cpu    = { x: GRID_W - 6, y: (GRID_H >> 1), d: 3, alive: true }; // facing left
  s.tickT = 0;
  s.roundOver = false;
  s.roundResetT = 0;
  s.turnQueued = null;        // 'left' | 'right' | null
  // claim spawn cells immediately so they count as trail
  s.grid[ix(s.player.x, s.player.y)] = PLAYER_TRAIL;
  s.grid[ix(s.cpu.x, s.cpu.y)] = CPU_TRAIL;
}

// ---- input ---------------------------------------------------------------
function turnPlayer(s, side) {
  if (s.over || s.roundOver || !s.player.alive) return;
  s.turnQueued = side;
}

function applyTurn(cycle, side) {
  if (side === 'left')  cycle.d = (cycle.d + 3) & 3;
  if (side === 'right') cycle.d = (cycle.d + 1) & 3;
}

// ---- helpers -------------------------------------------------------------
function inBounds(x, y) {
  return x >= 0 && y >= 0 && x < GRID_W && y < GRID_H;
}
function cellFree(grid, x, y) {
  return inBounds(x, y) && grid[ix(x, y)] === EMPTY;
}

// ---- AI ------------------------------------------------------------------
// pick the next direction for the CPU; returns its new `d`
function cpuChoose(s) {
  const c = s.cpu;
  const opts = [c.d, (c.d + 3) & 3, (c.d + 1) & 3];   // straight, left, right
  const safe = opts.filter(d => cellFree(s.grid, c.x + DIRS[d].dx, c.y + DIRS[d].dy));
  if (!safe.length) return c.d;                       // doomed
  if (s.cfg.ai === 'easy') {
    if (s.rng() < 0.07) return safe[(s.rng() * safe.length) | 0];
    return safe.includes(c.d) ? c.d : safe[0];
  }
  if (s.cfg.ai === 'medium') {
    // prefer the option that has more 1-step depth ahead
    let best = safe[0], bestScore = -1;
    for (const d of safe) {
      const nx = c.x + DIRS[d].dx, ny = c.y + DIRS[d].dy;
      let score = 1;
      for (let k = 0; k < 3; k++) {
        if (!cellFree(s.grid, nx + DIRS[d].dx * (k + 1), ny + DIRS[d].dy * (k + 1))) break;
        score++;
      }
      // prefer same-direction to keep flow
      if (d === c.d) score += 0.5;
      if (score > bestScore) { bestScore = score; best = d; }
    }
    return best;
  }
  if (s.cfg.ai === 'hard') {
    // hard: flood-fill space after the move, pick the option with the most room
    let best = safe[0], bestScore = -1;
    for (const d of safe) {
      const nx = c.x + DIRS[d].dx, ny = c.y + DIRS[d].dy;
      const score = floodArea(s.grid, nx, ny);
      if (score > bestScore) { bestScore = score; best = d; }
    }
    return best;
  }
  // elite: a Voronoi-style space-advantage search. For each safe move,
  // tentatively lay the CPU's trail, then compare the territory the CPU
  // can still reach against the territory the PLAYER can still reach, and
  // pick the move that maximises (cpuArea - playerArea) — it actively
  // walls the player out rather than just hoarding open room.
  const p = s.player;
  let best = safe[0], bestScore = -Infinity;
  for (const d of safe) {
    const nx = c.x + DIRS[d].dx, ny = c.y + DIRS[d].dy;
    const i = ix(nx, ny);
    const saved = s.grid[i];
    s.grid[i] = CPU_TRAIL;                       // tentatively occupy
    const cpuArea = floodArea(s.grid, nx, ny);
    // Player's likely next cell (straight ahead, else current cell).
    let px = p.x + DIRS[p.d].dx, py = p.y + DIRS[p.d].dy;
    if (!cellFree(s.grid, px, py)) { px = p.x; py = p.y; }
    const playerArea = floodArea(s.grid, px, py);
    s.grid[i] = saved;                           // restore
    const score = cpuArea - playerArea + (d === c.d ? 0.5 : 0);
    if (score > bestScore) { bestScore = score; best = d; }
  }
  return best;
}

function floodArea(grid, sx, sy) {
  if (!cellFree(grid, sx, sy)) return 0;
  const seen = new Uint8Array(GRID_W * GRID_H);
  const stack = [ix(sx, sy)];
  seen[stack[0]] = 1;
  let n = 0;
  while (stack.length) {
    const i = stack.pop();
    n++;
    const x = i % GRID_W, y = (i / GRID_W) | 0;
    for (const d of DIRS) {
      const nx = x + d.dx, ny = y + d.dy;
      if (!inBounds(nx, ny)) continue;
      const j = ix(nx, ny);
      if (seen[j] || grid[j] !== EMPTY) continue;
      seen[j] = 1; stack.push(j);
    }
  }
  return n;
}

// ---- tick ----------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  if (s.roundOver) {
    s.roundResetT -= dt;
    if (s.roundResetT <= 0) {
      // start a new round (unless match already decided)
      if (s.playerWins >= ROUNDS_TO_WIN || s.cpuWins >= ROUNDS_TO_WIN ||
          s.roundsPlayed >= MAX_ROUNDS) {
        s.over = true;
        s.won = s.playerWins > s.cpuWins;
        return;
      }
      startRound(s);
    }
    return;
  }
  s.tickT -= dt;
  while (s.tickT <= 0 && !s.roundOver) {
    s.tickT += TICK_INTERVAL;
    stepCycles(s);
  }
}

function stepCycles(s) {
  // apply player turn from the input queue
  if (s.turnQueued && s.player.alive) {
    applyTurn(s.player, s.turnQueued);
    s.turnQueued = null;
  }
  // cpu chooses
  if (s.cpu.alive) s.cpu.d = cpuChoose(s);
  // advance both
  const p = s.player, c = s.cpu;
  const pNext = { x: p.x + DIRS[p.d].dx, y: p.y + DIRS[p.d].dy };
  const cNext = { x: c.x + DIRS[c.d].dx, y: c.y + DIRS[c.d].dy };
  // head-on collision: both die
  if (p.alive && c.alive && pNext.x === cNext.x && pNext.y === cNext.y) {
    p.alive = c.alive = false;
  } else {
    if (p.alive) {
      if (!cellFree(s.grid, pNext.x, pNext.y)) p.alive = false;
      else { p.x = pNext.x; p.y = pNext.y; s.grid[ix(p.x, p.y)] = PLAYER_TRAIL; }
    }
    if (c.alive) {
      if (!cellFree(s.grid, cNext.x, cNext.y)) c.alive = false;
      else { c.x = cNext.x; c.y = cNext.y; s.grid[ix(c.x, c.y)] = CPU_TRAIL; }
    }
  }
  if (!p.alive || !c.alive) endRound(s);
}

function endRound(s) {
  s.roundOver = true;
  s.roundResetT = ROUND_DELAY;
  s.roundsPlayed++;
  if (s.player.alive && !s.cpu.alive) s.playerWins++;
  else if (!s.player.alive && s.cpu.alive) s.cpuWins++;
  // both dead -> tie (no points). MAX_ROUNDS caps stalemates.
}
