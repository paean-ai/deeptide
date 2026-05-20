// Pixel Qubert - hop diagonally around an isometric pyramid changing
// each cube's top to the level target colour. Avoid the enemy balls
// that bounce down the pyramid. Falling off the edge loses a life.
//
// Pyramid layout (rows 0..ROWS-1; row r has r+1 cubes).
//   row 0:                 [0,0]
//   row 1:             [1,0]  [1,1]
//   row 2:         [2,0]  [2,1]  [2,2]
//   ...
// Diagonal hops:
//   NE  (up-right) :  (r, c)  ->  (r-1, c)
//   NW  (up-left)  :  (r, c)  ->  (r-1, c-1)
//   SE  (down-right): (r, c)  ->  (r+1, c+1)
//   SW  (down-left) : (r, c)  ->  (r+1, c)

const VW = 360, VH = 480;
const ROWS = 7;
const CUBE_COUNT = (ROWS * (ROWS + 1)) / 2;     // = 28 for ROWS=7

// ---- levels ------------------------------------------------------------
// Each level: a list of colour stages (in hop-progression order) and an
// enemy-spawn cadence.
const LEVELS = [
  // 1. Aurora — single hop turns a cube its target colour.
  {
    name: ['Aurora', '极光'],
    stages: ['#ffd34a'],            // base -> gold
    enemyCd: 4.0,
    enemyDescend: 0.95,
  },
  // 2. Twilight — same, faster enemies.
  {
    name: ['Twilight', '暮光'],
    stages: ['#5fc06e'],
    enemyCd: 3.2,
    enemyDescend: 0.78,
  },
  // 3. Nebula — two-hop progression; first hop tints, second hop completes.
  {
    name: ['Nebula', '星云'],
    stages: ['#ff8fd0', '#bda6ff'],
    enemyCd: 3.0,
    enemyDescend: 0.72,
  },
  // 4. Galaxy — two-hop, faster.
  {
    name: ['Galaxy', '星系'],
    stages: ['#5fc0ff', '#5fc06e'],
    enemyCd: 2.6,
    enemyDescend: 0.60,
  },
  // 5. Comet — three-hop progression; finishing every cube takes longer.
  {
    name: ['Comet', '彗星'],
    stages: ['#f0a040', '#ff7a7a', '#bda6ff'],
    enemyCd: 2.4,
    enemyDescend: 0.55,
  },
  // 6. Singularity — finale: three-hop + the fastest enemies.
  {
    name: ['Singularity', '奇点'],
    stages: ['#5fc0ff', '#5fc06e', '#ff7a7a'],
    enemyCd: 2.0,
    enemyDescend: 0.46,
  },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function inBounds(r, c) {
  return r >= 0 && r < ROWS && c >= 0 && c <= r;
}

// Hop offsets per direction.
const HOPS = {
  ne: { dr: -1, dc:  0 },
  nw: { dr: -1, dc: -1 },
  se: { dr:  1, dc:  1 },
  sw: { dr:  1, dc:  0 },
};

function targetCellFrom(r, c, dir) {
  const h = HOPS[dir];
  return { r: r + h.dr, c: c + h.dc };
}

// ---- runtime state -----------------------------------------------------
// cubes: 2D array `cubes[r][c]` -> stage index (0..stages.length).
// 0 = untouched; stages.length = fully target.
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const cubes = [];
  for (let r = 0; r < ROWS; r++) {
    const row = [];
    for (let c = 0; c <= r; c++) row.push(0);
    cubes.push(row);
  }
  return {
    levelIndex, cfg: lv,
    cubes,
    stagesPerCube: lv.stages.length,
    player: { r: 0, c: 0, hopT: 0, hopFrom: null, alive: true, hitFlash: 0 },
    enemies: [],
    enemyT: lv.enemyCd * 0.7,
    score: 0,
    lives: 2,
    completed: 0,           // number of cubes at fully-target stage
    elapsed: 0,
    over: false, won: false,
    flash: 0,
  };
}

// ---- input -------------------------------------------------------------
function hop(s, dir) {
  if (s.over || !s.player.alive) return false;
  if (s.player.hopT > 0) return false;   // mid-hop
  const from = { r: s.player.r, c: s.player.c };
  const to   = targetCellFrom(from.r, from.c, dir);
  if (!inBounds(to.r, to.c)) {
    // Hopped off the edge of the pyramid.
    s.player.hopFrom = from;
    s.player.r = to.r; s.player.c = to.c;
    s.player.hopT = 0.35;
    s.player.falling = true;
    return true;
  }
  s.player.hopFrom = from;
  s.player.r = to.r; s.player.c = to.c;
  s.player.hopT = 0.22;
  s.player.falling = false;
  return true;
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.elapsed += dt;
  s.flash = Math.max(0, s.flash - dt);
  if (s.player.hitFlash > 0) s.player.hitFlash = Math.max(0, s.player.hitFlash - dt);
  // Advance the player hop animation.
  if (s.player.hopT > 0) {
    s.player.hopT -= dt;
    if (s.player.hopT <= 0) {
      s.player.hopT = 0;
      if (s.player.falling) {
        die(s);
        return;
      }
      // Landed on a cube — advance its stage.
      const cell = s.cubes[s.player.r][s.player.c];
      if (cell < s.stagesPerCube) {
        s.cubes[s.player.r][s.player.c] = cell + 1;
        s.score += 25;
        if (s.cubes[s.player.r][s.player.c] === s.stagesPerCube) {
          s.completed++;
          s.score += 25;     // bonus on stage-complete
          if (s.completed === CUBE_COUNT) {
            s.over = true; s.won = true;
            s.score += 500;
            s.flash = 0.55;
            return;
          }
        }
      }
    }
  }
  // Enemy spawn.
  s.enemyT -= dt;
  if (s.enemyT <= 0) {
    spawnEnemy(s);
    s.enemyT = s.cfg.enemyCd * (0.8 + Math.random() * 0.4);
  }
  // Enemies descend.
  for (const e of s.enemies) {
    if (e.hopT > 0) {
      e.hopT -= dt;
      if (e.hopT <= 0) {
        e.r = e.nextR; e.c = e.nextC;
        if (e.r >= ROWS) { e.dead = true; continue; }
        // Pick next descent direction at random.
        const dir = Math.random() < 0.5 ? 'se' : 'sw';
        const to = targetCellFrom(e.r, e.c, dir);
        e.nextR = to.r; e.nextC = to.c;
        e.hopT = s.cfg.enemyDescend;
      }
    }
  }
  s.enemies = s.enemies.filter(e => !e.dead);
  // Enemy vs player collision (cell-level).
  if (s.player.alive && s.player.hopT === 0) {
    for (const e of s.enemies) {
      const er = e.hopT > 0 ? e.r : e.r;
      const ec = e.hopT > 0 ? e.c : e.c;
      if (er === s.player.r && ec === s.player.c) {
        die(s);
        return;
      }
    }
  }
}

function spawnEnemy(s) {
  // Spawns at top, descends.
  const dir = Math.random() < 0.5 ? 'se' : 'sw';
  const to = targetCellFrom(0, 0, dir);
  s.enemies.push({
    r: 0, c: 0,
    nextR: to.r, nextC: to.c,
    hopT: s.cfg.enemyDescend,
    color: '#ff5a5a',
  });
}

function die(s) {
  if (!s.player.alive) return;
  s.player.alive = false;
  s.player.hitFlash = 0.5;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  s.player.respawn = 0.6;
}

function resetPlayer(s) {
  s.player.r = 0; s.player.c = 0;
  s.player.hopT = 0; s.player.hopFrom = null;
  s.player.alive = true;
  s.player.falling = false;
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 75;
}

// Choose hop dir from a tap position relative to the player's screen pos.
// dx/dy are pixel offsets. Returns one of 'ne','nw','se','sw' or null for dead-zone.
function dirFromTap(dx, dy) {
  if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return null;
  if (dy < 0) return dx > 0 ? 'ne' : 'nw';
  return dx > 0 ? 'se' : 'sw';
}
