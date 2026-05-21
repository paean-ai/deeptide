// Pixel Twirl - a rotate-the-blocks picture puzzle (the Twiddle puzzle).
//
// Tap a 2x2 junction and that block of four tiles spins a quarter turn. The
// board starts scrambled; twirl it back into the target picture. Par is the
// scramble length - undo the scramble to match it, or find a shorter route.

const VW = 360, VH = 480;

// Each level: a procedural target pattern, palette size, scramble seed/depth.
const LEVELS = [
  { name: ['Pinwheel', '风车'], cols: 4, rows: 4, pattern: 'quad',  colors: 4, seed: 17,  depth: 6 },
  { name: ['Eddy', '涡流'],     cols: 5, rows: 4, pattern: 'diag',  colors: 3, seed: 53,  depth: 9 },
  { name: ['Vortex', '漩涡'],   cols: 5, rows: 5, pattern: 'rings', colors: 3, seed: 131, depth: 13 },
  { name: ['Cyclone', '气旋'],  cols: 6, rows: 5, pattern: 'bands', colors: 4, seed: 247, depth: 17 },
  { name: ['Maelstrom', '巨涡'], cols: 6, rows: 6, pattern: 'cross', colors: 3, seed: 389, depth: 22 },
  { name: ['Galaxy', '星旋'],   cols: 6, rows: 6, pattern: 'rings', colors: 4, seed: 547, depth: 28 },
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

function patternColor(L, r, c) {
  const C = L.colors;
  switch (L.pattern) {
    case 'rings': {
      const cr = (L.rows - 1) / 2, cc = (L.cols - 1) / 2;
      return Math.min(C - 1, Math.max(Math.abs(r - cr), Math.abs(c - cc)) | 0);
    }
    case 'diag': return (r + c) % C;
    case 'quad': return (r < L.rows / 2 ? 0 : 2) + (c < L.cols / 2 ? 0 : 1);
    case 'bands': return Math.min(C - 1, (r * C / L.rows) | 0);
    case 'cross': {
      const cr = (L.rows - 1) / 2 | 0, cc = (L.cols - 1) / 2 | 0;
      if (r === cr || c === cc) return 0;
      return 1 + ((r + c) % 2);
    }
    default: return 0;
  }
}

function buildTarget(L) {
  const g = new Array(L.cols * L.rows);
  for (let r = 0; r < L.rows; r++) {
    for (let c = 0; c < L.cols; c++) g[r * L.cols + c] = patternColor(L, r, c);
  }
  return g;
}

// rotate the 2x2 block whose top-left is (r,c). dir +1 clockwise, -1 anti.
function rotateBlock(grid, cols, r, c, dir) {
  const i0 = r * cols + c, i1 = r * cols + c + 1;
  const i2 = (r + 1) * cols + c, i3 = (r + 1) * cols + c + 1;
  const a = grid[i0], b = grid[i1], d = grid[i2], e = grid[i3];
  if (dir > 0) {                      // CW: TL<-BL, TR<-TL, BR<-TR, BL<-BR
    grid[i1] = a; grid[i3] = b; grid[i2] = e; grid[i0] = d;
  } else {                            // CCW
    grid[i2] = a; grid[i3] = d; grid[i1] = e; grid[i0] = b;
  }
}

// scramble a copy of the target; returns { grid, seq }. Avoids spinning the
// same block straight back, so the depth is an honest par.
function scramble(L) {
  const grid = buildTarget(L);
  const rng = seededRandom(L.seed);
  const seq = [];
  let prev = null;
  for (let i = 0; i < L.depth; i++) {
    let m;
    for (let tries = 0; tries < 24; tries++) {
      const r = (rng() * (L.rows - 1)) | 0;
      const c = (rng() * (L.cols - 1)) | 0;
      const dir = rng() < 0.5 ? 1 : -1;
      m = { r, c, dir };
      if (!prev || !(prev.r === r && prev.c === c && prev.dir === -dir)) break;
    }
    rotateBlock(grid, L.cols, m.r, m.c, m.dir);
    seq.push(m);
    prev = m;
  }
  return { grid, seq };
}

// ---- play state ----------------------------------------------------------
function newPlay(levelIndex) {
  const level = LEVELS[levelIndex];
  const target = buildTarget(level);
  const sc = scramble(level);
  return {
    levelIndex, level, cols: level.cols, rows: level.rows,
    target,
    start: sc.grid.slice(),
    scrambleSeq: sc.seq,
    grid: sc.grid.slice(),
    moves: 0, history: [], over: false, won: false,
  };
}

function gridMatches(a, b) {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}
function isWon(s) { return gridMatches(s.grid, s.target); }

// twirl the 2x2 block at junction (r,c). returns true if a turn was taken.
function doTwirl(s, r, c, dir) {
  if (s.over) return false;
  if (r < 0 || c < 0 || r >= s.rows - 1 || c >= s.cols - 1) return false;
  s.history.push(s.grid.slice());
  rotateBlock(s.grid, s.cols, r, c, dir);
  s.moves++;
  if (isWon(s)) { s.over = true; s.won = true; }
  return true;
}

function undo(s) {
  if (!s.history.length) return false;
  s.grid = s.history.pop();
  s.moves--;
  s.over = false; s.won = false;
  return true;
}

function restart(s) {
  s.grid = s.start.slice();
  s.moves = 0; s.history = []; s.over = false; s.won = false;
}

function stars(moves, par) {
  if (moves <= par) return 3;
  if (moves <= Math.round(par * 1.8)) return 2;
  return 1;
}
