// Pixel Sumplete - the keep-or-delete sum puzzle.
//
// Rules:
//   * A grid is filled with positive integers.
//   * Row + column targets sit beside the grid.
//   * For every cell, decide KEEP or DELETE — the SUM of the KEPT cells in
//     each row must equal the row's target, and same for every column.
//   * Find the unique kept-mask.
//
// Each level is { n, seed }. buildPuzzle is deterministic per seed and
// returns { n, grid, kept (solution), rowT, colT, levelIndex, cfg }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Spark',    '火花'], n: 4, seed: 131 },
  { name: ['Ember',    '余烬'], n: 4, seed: 287 },
  { name: ['Furnace',  '炉膛'], n: 5, seed: 431 },
  { name: ['Forge',    '锻台'], n: 5, seed: 531 },
  { name: ['Smelter',  '熔炉'], n: 6, seed: 631 },
  { name: ['Crucible', '坩埚'], n: 6, seed: 733 },
];
const LEVEL_COUNT = LEVELS.length;

// Live cell tags (player marks):
const UNDECIDED = 0;
const KEEP      = 1;
const DELETE    = 2;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// One attempt: random grid of 1..n + random kept-mask -> derive targets.
function tryPuzzle(n, seed) {
  const rng = seededRandom(seed);
  const grid = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(1 + ((rng() * n) | 0));
    grid.push(row);
  }
  const kept = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(rng() < 0.55 ? 1 : 0);
    kept.push(row);
  }
  const rowT = new Array(n).fill(0);
  const colT = new Array(n).fill(0);
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) {
    if (kept[r][c]) { rowT[r] += grid[r][c]; colT[c] += grid[r][c]; }
  }
  return { grid, kept, rowT, colT };
}

// Count solutions of (grid, rowT, colT). For each row enumerate subsets
// summing to rowT[r] (2^n masks), prune by column over-sum.
function solveCount(n, grid, rowT, colT, limit) {
  let count = 0;
  const colSum = new Array(n).fill(0);
  function rec(r) {
    if (count >= limit) return;
    if (r === n) {
      for (let c = 0; c < n; c++) if (colSum[c] !== colT[c]) return;
      count++;
      return;
    }
    const N = 1 << n;
    for (let mask = 0; mask < N; mask++) {
      let sum = 0;
      for (let c = 0; c < n; c++) if ((mask >> c) & 1) sum += grid[r][c];
      if (sum !== rowT[r]) continue;
      let ok = true;
      for (let c = 0; c < n; c++) {
        if ((mask >> c) & 1) {
          colSum[c] += grid[r][c];
          if (colSum[c] > colT[c]) ok = false;
        }
      }
      if (ok) rec(r + 1);
      for (let c = 0; c < n; c++) if ((mask >> c) & 1) colSum[c] -= grid[r][c];
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const { n, seed } = cfg;
  for (let attempt = 0; attempt < 60; attempt++) {
    const p = tryPuzzle(n, seed + attempt * 1009);
    if (solveCount(n, p.grid, p.rowT, p.colT, 2) === 1) {
      return Object.assign({ n, levelIndex, cfg }, p);
    }
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// `marks` is the live grid (UNDECIDED / KEEP / DELETE per cell).
// Returns the set of cell indices currently in conflict:
//   * a row whose KEEP cells sum > row target.
//   * a row whose KEEP-or-UNDECIDED cells sum < row target.
//   * same for columns.
// (The exact `=` test happens at solve time.)
function findViolations(n, grid, rowT, colT, marks) {
  const bad = new Set();
  for (let r = 0; r < n; r++) {
    let keepSum = 0, maxSum = 0;
    for (let c = 0; c < n; c++) {
      if (marks[r][c] === KEEP)     { keepSum += grid[r][c]; maxSum += grid[r][c]; }
      else if (marks[r][c] === UNDECIDED) maxSum += grid[r][c];
    }
    if (keepSum > rowT[r] || maxSum < rowT[r]) {
      for (let c = 0; c < n; c++) if (marks[r][c] !== DELETE) bad.add(r * n + c);
    }
  }
  for (let c = 0; c < n; c++) {
    let keepSum = 0, maxSum = 0;
    for (let r = 0; r < n; r++) {
      if (marks[r][c] === KEEP)     { keepSum += grid[r][c]; maxSum += grid[r][c]; }
      else if (marks[r][c] === UNDECIDED) maxSum += grid[r][c];
    }
    if (keepSum > colT[c] || maxSum < colT[c]) {
      for (let r = 0; r < n; r++) if (marks[r][c] !== DELETE) bad.add(r * n + c);
    }
  }
  return bad;
}

function isSolved(n, grid, rowT, colT, marks) {
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (marks[r][c] === UNDECIDED) return false;
  // Row + column kept-sums match.
  for (let r = 0; r < n; r++) {
    let s = 0;
    for (let c = 0; c < n; c++) if (marks[r][c] === KEEP) s += grid[r][c];
    if (s !== rowT[r]) return false;
  }
  for (let c = 0; c < n; c++) {
    let s = 0;
    for (let r = 0; r < n; r++) if (marks[r][c] === KEEP) s += grid[r][c];
    if (s !== colT[c]) return false;
  }
  return true;
}
