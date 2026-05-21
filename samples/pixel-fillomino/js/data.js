// Pixel Fillomino - the region-size logic puzzle. Pure logic + baked levels.
//
// Fill every cell with a number. A cell holding N is part of a block of
// exactly N orthogonally-connected cells all holding N, and two blocks of the
// same size may never touch edge to edge. The 6 levels are generated and
// uniqueness-verified offline (a backtracking region solver) and baked here;
// the test re-derives that each clued grid has exactly one solution.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Cottage', '小屋'], n: 5,
    clues: [2,1,6,8,0,0,6,0,8,0,0,6,6,8,8,3,3,3,1,8,4,4,0,4,0],
    solution: [2,1,6,8,8,2,6,6,8,8,6,6,6,8,8,3,3,3,1,8,4,4,4,4,8] },
  { name: ['Hamlet', '村落'], n: 6,
    clues: [8,8,8,8,4,4,8,0,8,8,4,4,8,0,4,4,3,3,2,1,6,0,1,0,0,6,0,6,0,1,1,3,0,3,0,2],
    solution: [8,8,8,8,4,4,8,4,8,8,4,4,8,4,4,4,3,3,2,1,6,6,1,3,2,6,6,6,6,1,1,3,3,3,2,2] },
  { name: ['Orchard', '果园'], n: 6,
    clues: [4,4,4,2,1,2,6,1,0,2,3,2,0,6,6,0,3,1,2,2,6,1,0,2,3,0,4,4,0,2,3,2,0,4,4,0],
    solution: [4,4,4,2,1,2,6,1,4,2,3,2,6,6,6,6,3,1,2,2,6,1,3,2,3,3,4,4,1,2,3,2,2,4,4,1] },
  { name: ['Township', '城邑'], n: 7,
    clues: [6,0,0,4,0,4,0,0,6,0,3,3,0,4,6,8,8,3,1,4,4,8,0,8,1,0,0,1,0,0,0,6,0,0,0,3,3,0,4,2,6,2,3,0,4,1,2,0,1],
    solution: [6,6,4,4,4,4,1,6,6,6,3,3,1,4,6,8,8,3,1,4,4,8,8,8,1,6,4,1,8,8,8,6,6,6,2,3,3,4,4,2,6,2,3,4,4,1,2,6,1] },
  { name: ['Borough', '市镇'], n: 7,
    clues: [0,0,8,0,6,6,0,0,8,8,0,6,1,0,3,3,3,6,0,8,8,0,8,0,0,6,8,8,8,1,0,8,0,3,8,1,0,0,4,4,3,8,2,0,1,0,4,2,2],
    solution: [8,8,8,8,6,6,8,8,8,8,8,6,1,8,3,3,3,6,6,8,8,8,8,8,1,6,8,8,8,1,8,8,3,3,8,1,8,8,4,4,3,8,2,2,1,4,4,2,2] },
  { name: ['Citadel', '城郭'], n: 7,
    clues: [0,0,8,1,8,8,6,0,0,0,0,8,2,6,2,1,4,4,0,2,0,1,0,0,1,0,6,0,0,6,6,0,0,1,6,0,6,1,0,3,8,8,2,0,6,8,0,0,0],
    solution: [1,8,8,1,8,8,6,2,8,8,8,8,2,6,2,1,4,4,1,2,6,1,4,4,1,3,6,6,6,6,6,8,3,1,6,2,6,1,8,3,8,8,2,6,6,8,8,8,8] },
];
const LEVEL_COUNT = LEVELS.length;
const MAX_NUM = 8;

const NB = [[-1, 0], [1, 0], [0, -1], [0, 1]];
function neighbors(n, c) {
  const r = c / n | 0, col = c % n, out = [];
  for (const [dr, dc] of NB) {
    const nr = r + dr, nc = col + dc;
    if (nr >= 0 && nc >= 0 && nr < n && nc < n) out.push(nr * n + nc);
  }
  return out;
}

// ---- play state ----------------------------------------------------------
function newPlay(levelIndex) {
  const level = LEVELS[levelIndex];
  return {
    levelIndex, level, n: level.n,
    clues: level.clues,
    grid: level.clues.slice(),     // player grid; clued cells pre-filled
    over: false, won: false,
  };
}
function isGiven(s, idx) { return s.clues[idx] !== 0; }

// set a player cell (val 0 erases). clued cells are immutable.
function setCell(s, idx, val) {
  if (s.over || isGiven(s, idx)) return false;
  if (val < 0 || val > MAX_NUM) return false;
  s.grid[idx] = val;
  if (isSolved(s)) { s.over = true; s.won = true; }
  return true;
}

// cells in conflict: a block bigger than its number, or a sealed-in block
// smaller than its number.
function findViolations(n, grid) {
  const N = n * n, seen = new Array(N).fill(false), bad = new Set();
  for (let i = 0; i < N; i++) {
    if (seen[i] || grid[i] === 0) continue;
    const v = grid[i], comp = [i], stack = [i];
    seen[i] = true;
    while (stack.length) {
      const x = stack.pop();
      for (const nb of neighbors(n, x)) {
        if (!seen[nb] && grid[nb] === v) { seen[nb] = true; comp.push(nb); stack.push(nb); }
      }
    }
    let openEdge = false;
    for (const x of comp) {
      for (const nb of neighbors(n, x)) if (grid[nb] === 0) openEdge = true;
    }
    if (comp.length > v || (comp.length < v && !openEdge)) {
      for (const x of comp) bad.add(x);
    }
  }
  return bad;
}

function isSolved(s) {
  for (let i = 0; i < s.grid.length; i++) if (s.grid[i] === 0) return false;
  return findViolations(s.n, s.grid).size === 0;
}

function restart(s) {
  s.grid = s.clues.slice();
  s.over = false; s.won = false;
}

function filledCount(s) {
  let k = 0;
  for (const v of s.grid) if (v !== 0) k++;
  return k;
}
