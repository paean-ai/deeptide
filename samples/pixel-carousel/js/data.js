// Pixel Carousel - a wrap-shift picture puzzle (Loopover family).
//
// Every row and every column is a carousel: shift it and the tiles cycle
// round, wrapping edge to edge. The board starts scrambled; restore the
// target picture. Par is the scramble length - you can always match it by
// undoing the scramble, and a sharp eye can beat it.

const VW = 360, VH = 480;

// Each level: a procedural target pattern, palette size, scramble seed and
// scramble depth (which is par). cols x rows.
const LEVELS = [
  { name: ['Spark', '火花'],   cols: 3, rows: 3, pattern: 'rings', colors: 2, seed: 17,  depth: 5 },
  { name: ['Drift', '流光'],   cols: 4, rows: 3, pattern: 'diag',  colors: 3, seed: 53,  depth: 8 },
  { name: ['Quarter', '四分'], cols: 4, rows: 4, pattern: 'quad',  colors: 4, seed: 131, depth: 12 },
  { name: ['Strata', '层叠'],  cols: 5, rows: 4, pattern: 'bands', colors: 4, seed: 247, depth: 16 },
  { name: ['Halo', '光环'],    cols: 5, rows: 5, pattern: 'rings', colors: 3, seed: 389, depth: 20 },
  { name: ['Crossfire', '交火'], cols: 6, rows: 5, pattern: 'cross', colors: 3, seed: 547, depth: 26 },
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

// the target colour of cell (r,c) for a level's pattern
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

// ---- the carousel shifts -------------------------------------------------
// dir +1 shifts a row right / a column down; -1 the other way.
function shiftRow(grid, cols, rows, r, dir) {
  const row = [];
  for (let c = 0; c < cols; c++) row.push(grid[r * cols + c]);
  for (let c = 0; c < cols; c++) {
    grid[r * cols + c] = row[((c - dir) % cols + cols) % cols];
  }
}
function shiftCol(grid, cols, rows, c, dir) {
  const col = [];
  for (let r = 0; r < rows; r++) col.push(grid[r * cols + c]);
  for (let r = 0; r < rows; r++) {
    grid[r * cols + c] = col[((r - dir) % rows + rows) % rows];
  }
}

// scramble a copy of the target; returns { grid, seq } where seq is the list
// of shifts applied (so a test can invert it). Avoids immediately undoing the
// previous shift so the depth is an honest par.
function scramble(L) {
  const grid = buildTarget(L);
  const rng = seededRandom(L.seed);
  const seq = [];
  let prev = null;
  for (let i = 0; i < L.depth; i++) {
    let m;
    for (let tries = 0; tries < 20; tries++) {
      const axis = rng() < 0.5 ? 'row' : 'col';
      const idx = (rng() * (axis === 'row' ? L.rows : L.cols)) | 0;
      const dir = rng() < 0.5 ? 1 : -1;
      m = { axis, idx, dir };
      if (!prev || !(prev.axis === axis && prev.idx === idx && prev.dir === -dir)) break;
    }
    if (m.axis === 'row') shiftRow(grid, L.cols, L.rows, m.idx, m.dir);
    else shiftCol(grid, L.cols, L.rows, m.idx, m.dir);
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

// apply a shift as a player move. axis 'row'|'col'.
function doShift(s, axis, idx, dir) {
  if (s.over) return false;
  s.history.push(s.grid.slice());
  if (axis === 'row') shiftRow(s.grid, s.cols, s.rows, idx, dir);
  else shiftCol(s.grid, s.cols, s.rows, idx, dir);
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
  if (moves <= Math.round(par * 1.7)) return 2;
  return 1;
}
