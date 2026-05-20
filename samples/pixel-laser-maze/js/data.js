// Pixel Laser Maze - level grids, beam tracing, solvability checking.

const VW = 360, VH = 480;

// Grid chars: '.' empty  '#' wall  '>'/'<'/'^'/'v' emitter  'T' crystal target.
// mirrors = how many mirrors the player may place.
const RAW_LEVELS = [
  { mirrors: 1, grid: [
    '........', '........', '........', '>.......',
    '........', '........', '........', '....T...',
  ] },
  { mirrors: 2, grid: [
    '........', '>...#...', '........', '........',
    '....T...', '........', '........', '........',
  ] },
  { mirrors: 2, grid: [
    '........', '>..T....', '....#...', '........',
    '.T......', '........', '........', '........',
  ] },
  { mirrors: 3, grid: [
    '...v....', '........', '........', '#.......',
    '........', '........', '........', '.....T..',
  ] },
  { mirrors: 3, grid: [
    '>...#...', '........', '........', '....#...',
    '........', '........', '........', '....T...',
  ] },
  { mirrors: 3, grid: [
    '>...T...', '........', '........', '........',
    '...T....', '........', '........', '........',
  ] },
  { mirrors: 4, grid: [
    '>......#', '........', '#.......', '.......#',
    '........', '#.......', '........', '...T....',
  ] },
  { mirrors: 4, grid: [
    '...v....', '........', '........', '.T......',
    '........', '........', '......T.', '........',
  ] },
  { mirrors: 3, grid: [
    '>...#...', '........', '........', '........',
    '........', '........', '........', '....T...',
  ] },
  { mirrors: 3, grid: [
    '>..T...#', '........', '........', '......#.',
    '........', 'T.......', '........', '........',
  ] },
  { mirrors: 4, grid: [
    '...v....', '........', '#.......', '........',
    '........', '.......#', '........', '..T.....',
  ] },
  { mirrors: 4, grid: [
    '>.......', '........', '....T...', '........',
    '........', '........', '........', '.....T..',
  ] },
  { mirrors: 3, grid: [
    '>.......', '........', '.......T', '........',
    '....#...', '........', 'T.......', '........',
  ] },
  { mirrors: 3, grid: [
    '>....#..', '........', '........', '........',
    '........', '........', '........', '......T.',
  ] },
  { mirrors: 4, grid: [
    '...v....', '........', 'T.......', '#.......',
    '........', '........', '......T.', '........',
  ] },
  { mirrors: 4, grid: [
    '>....#..', '........', '....#...', '........',
    '.....T..', '........', '........', 'T.......',
  ] },
];

const EMIT_DIR = { '>': [1, 0], '<': [-1, 0], '^': [0, -1], 'v': [0, 1] };

// Parse a raw grid into a structured level.
function makeLevel(raw) {
  const grid = raw.grid;
  const rows = grid.length, cols = grid[0].length;
  let emitter = null, edir = null;
  const targets = [], walls = new Set(), empties = [];
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const ch = grid[y][x];
      if (EMIT_DIR[ch]) { emitter = [x, y]; edir = EMIT_DIR[ch]; }
      else if (ch === 'T') targets.push([x, y]);
      else if (ch === '#') walls.add(x + ',' + y);
      else empties.push(x + ',' + y);
    }
  }
  return {
    grid, rows, cols, emitter, edir, targets, walls, empties,
    targetSet: new Set(targets.map(t => t[0] + ',' + t[1])),
    mirrors: raw.mirrors,
  };
}
const LEVELS = RAW_LEVELS.map(makeLevel);
const LEVEL_COUNT = LEVELS.length;

// Trace the beam; returns the set of target keys the beam passes through.
// `mirrors` is a Map of "x,y" -> '/' | '\\'.
function traceBeam(L, mirrors) {
  let x = L.emitter[0], y = L.emitter[1];
  let dx = L.edir[0], dy = L.edir[1];
  const lit = new Set();
  const cap = L.cols * L.rows * 4;
  for (let step = 0; step < cap; step++) {
    x += dx; y += dy;
    if (x < 0 || y < 0 || x >= L.cols || y >= L.rows) break;
    const key = x + ',' + y;
    if (L.walls.has(key)) break;
    if (L.targetSet.has(key)) lit.add(key);
    const m = mirrors.get(key);
    if (m === '/') { const t = -dy; dy = -dx; dx = t; }
    else if (m === '\\') { const t = dy; dy = dx; dx = t; }
  }
  return lit;
}
function beamWins(L, mirrors) {
  const lit = traceBeam(L, mirrors);
  return L.targets.every(t => lit.has(t[0] + ',' + t[1]));
}

// Brute-force search: can the level be solved with <= L.mirrors mirrors?
function levelSolvable(L) {
  const e = L.empties;
  let nodes = 0;
  function rec(start, placed, count) {
    if (++nodes > 3000000) return false;
    if (beamWins(L, placed)) return true;
    if (count === 0) return false;
    for (let i = start; i < e.length; i++) {
      for (const m of ['/', '\\']) {
        placed.set(e[i], m);
        if (rec(i + 1, placed, count - 1)) return true;
        placed.delete(e[i]);
      }
    }
    return false;
  }
  return rec(0, new Map(), L.mirrors);
}
