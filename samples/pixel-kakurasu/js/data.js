// Pixel Kakurasu - index-sum logic puzzle.
//
// Rules:
//   * A row's "score" is the SUM OF COLUMN INDICES of the shaded cells in
//     that row (column indices start at 1).
//   * A column's score is the SUM OF ROW INDICES of the shaded cells in
//     that column.
//   * Row + column targets are printed beside the grid. Shade cells so all
//     row scores and all column scores match.
//   * Find the unique shading.
//
// Each level is { n, seed }. buildPuzzle is deterministic per seed and
// returns { n, shade (solution), rowT, colT, levelIndex, cfg }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Brook',   '小溪'], n: 4, seed: 131 },
  { name: ['Stream',  '溪流'], n: 4, seed: 287 },
  { name: ['River',   '江河'], n: 5, seed: 431 },
  { name: ['Lake',    '湖泊'], n: 5, seed: 531 },
  { name: ['Sea',     '大海'], n: 6, seed: 631 },
  { name: ['Ocean',   '汪洋'], n: 6, seed: 733 },
];
const LEVEL_COUNT = LEVELS.length;

// Live cell tags:
const UNDECIDED = 0;
const SHADED    = 1;
const EMPTY     = 2;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// One attempt: random shading -> derive targets.
function tryPuzzle(n, seed) {
  const rng = seededRandom(seed);
  const shade = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(rng() < 0.5 ? 1 : 0);
    shade.push(row);
  }
  const rowT = new Array(n).fill(0);
  const colT = new Array(n).fill(0);
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) {
    if (shade[r][c]) { rowT[r] += (c + 1); colT[c] += (r + 1); }
  }
  return { shade, rowT, colT };
}

// Count solutions of (rowT, colT). Row-by-row subset enumeration with col
// over-sum pruning, leaf exact-match column verification.
function solveCount(n, rowT, colT, limit) {
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
      for (let c = 0; c < n; c++) if ((mask >> c) & 1) sum += (c + 1);
      if (sum !== rowT[r]) continue;
      let ok = true;
      for (let c = 0; c < n; c++) {
        if ((mask >> c) & 1) {
          colSum[c] += (r + 1);
          if (colSum[c] > colT[c]) ok = false;
        }
      }
      if (ok) rec(r + 1);
      for (let c = 0; c < n; c++) if ((mask >> c) & 1) colSum[c] -= (r + 1);
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
    if (solveCount(n, p.rowT, p.colT, 2) === 1) {
      return Object.assign({ n, levelIndex, cfg }, p);
    }
  }
  return null;
}

// ---- live validation ---------------------------------------------------
// `marks` is the live grid (UNDECIDED / SHADED / EMPTY per cell).
// Conflicts: any row whose SHADED row-sum > rowT (locked overshoot) OR
// whose (SHADED + UNDECIDED) row-sum < rowT (already-locked undershoot).
// Same for columns. The exact `=` test is in `isSolved`.
function findViolations(n, rowT, colT, marks) {
  const bad = new Set();
  for (let r = 0; r < n; r++) {
    let lockSum = 0, maxSum = 0;
    for (let c = 0; c < n; c++) {
      if (marks[r][c] === SHADED)    { lockSum += (c + 1); maxSum += (c + 1); }
      else if (marks[r][c] === UNDECIDED) maxSum += (c + 1);
    }
    if (lockSum > rowT[r] || maxSum < rowT[r]) {
      for (let c = 0; c < n; c++) if (marks[r][c] !== EMPTY) bad.add(r * n + c);
    }
  }
  for (let c = 0; c < n; c++) {
    let lockSum = 0, maxSum = 0;
    for (let r = 0; r < n; r++) {
      if (marks[r][c] === SHADED)    { lockSum += (r + 1); maxSum += (r + 1); }
      else if (marks[r][c] === UNDECIDED) maxSum += (r + 1);
    }
    if (lockSum > colT[c] || maxSum < colT[c]) {
      for (let r = 0; r < n; r++) if (marks[r][c] !== EMPTY) bad.add(r * n + c);
    }
  }
  return bad;
}

function isSolved(n, rowT, colT, marks) {
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (marks[r][c] === UNDECIDED) return false;
  for (let r = 0; r < n; r++) {
    let s = 0;
    for (let c = 0; c < n; c++) if (marks[r][c] === SHADED) s += (c + 1);
    if (s !== rowT[r]) return false;
  }
  for (let c = 0; c < n; c++) {
    let s = 0;
    for (let r = 0; r < n; r++) if (marks[r][c] === SHADED) s += (r + 1);
    if (s !== colT[c]) return false;
  }
  return true;
}
