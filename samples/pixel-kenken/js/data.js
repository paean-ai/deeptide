// Pixel KenKen - Latin square + arithmetic cages (KenKen / Calcudoku).
//
// Rules:
//   * Fill every cell with a digit 1..n.
//   * Each row and column contains every digit exactly once (Latin square).
//   * The grid is divided into "cages". Each cage has a target value and an
//     operator (+, ×, -, ÷, or = for a single cell). The cells in the cage
//     must satisfy that arithmetic — sum, product, |a-b|, or larger/smaller.
//   * Order doesn't matter inside a cage. Repeats are allowed in a cage as
//     long as the Latin-square rule isn't broken.
//
// Each level is { n, seed }. buildPuzzle is deterministic per seed and
// returns { n, solution, cages, levelIndex, cfg }.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Foothills', '丘陵'], n: 4, seed: 131 },
  { name: ['Highland',  '高地'], n: 4, seed: 287 },
  { name: ['Plateau',   '台地'], n: 5, seed: 431 },
  { name: ['Mesa',      '方山'], n: 5, seed: 531 },
  { name: ['Ridge',     '山脊'], n: 6, seed: 631 },
  { name: ['Summit',    '巅峰'], n: 6, seed: 733 },
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

// Random Latin square 1..n by row-shift + row/col/value permutation.
function randomLatinSquare(n, rng) {
  const base = [];
  for (let i = 0; i < n; i++) base.push(i);
  const rowPerm = base.slice();
  const colPerm = base.slice();
  for (let i = n - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [rowPerm[i], rowPerm[j]] = [rowPerm[j], rowPerm[i]];
    [colPerm[i], colPerm[j]] = [colPerm[j], colPerm[i]];
  }
  const valPerm = base.slice().map(v => v + 1);
  for (let i = n - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [valPerm[i], valPerm[j]] = [valPerm[j], valPerm[i]];
  }
  const grid = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(valPerm[(rowPerm[r] + colPerm[c]) % n]);
    grid.push(row);
  }
  return grid;
}

// BFS-grow random cages of size 1..maxSize. Standard partition pattern.
function partitionIntoCages(n, rng, maxSize) {
  const total = n * n;
  const owner = new Array(total).fill(-1);
  const regions = [];
  while (owner.some(v => v === -1)) {
    const empties = [];
    for (let i = 0; i < total; i++) if (owner[i] === -1) empties.push(i);
    const start = empties[(rng() * empties.length) | 0];
    const target = 1 + ((rng() * maxSize) | 0);
    const cells = [start];
    owner[start] = regions.length;
    while (cells.length < target) {
      const cand = [];
      for (const c of cells) {
        const x = c % n, y = (c / n) | 0;
        for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const nx = x + dx, ny = y + dy;
          if (nx<0||nx>=n||ny<0||ny>=n) continue;
          const ni = ny * n + nx;
          if (owner[ni] === -1) cand.push(ni);
        }
      }
      if (!cand.length) break;
      const pick = cand[(rng() * cand.length) | 0];
      cells.push(pick); owner[pick] = regions.length;
    }
    regions.push(cells);
  }
  return regions;
}

// Pick an op + target for a cage, using its solution values.
function makeCage(cells, solution, n, rng) {
  const vals = cells.map(c => solution[(c / n) | 0][c % n]);
  if (vals.length === 1) return { cells, op: '=', target: vals[0] };
  if (vals.length === 2) {
    const [a, b] = vals;
    const lo = Math.min(a, b), hi = Math.max(a, b);
    const ops = [];
    ops.push({ op: '+', target: a + b });
    ops.push({ op: '×', target: a * b });
    ops.push({ op: '-', target: hi - lo });
    if (lo > 0 && hi % lo === 0) ops.push({ op: '÷', target: hi / lo });
    return Object.assign({ cells }, ops[(rng() * ops.length) | 0]);
  }
  // 3+ cells: sum or product only (KenKen convention).
  if (rng() < 0.5) return { cells, op: '+', target: vals.reduce((a, b) => a + b, 0) };
  return { cells, op: '×', target: vals.reduce((a, b) => a * b, 1) };
}

// Count solutions of (n, cages) up to `limit`. Backtrack cell-by-cell with
// Latin-square pruning and per-cage early checks (partial +, × prune; exact
// match on cage completion).
function solveCount(n, cages, limit) {
  const grid = [];
  for (let r = 0; r < n; r++) grid.push(new Array(n).fill(0));
  const cellCage = new Array(n * n).fill(-1);
  cages.forEach((cg, i) => cg.cells.forEach(c => cellCage[c] = i));
  let count = 0;
  function valid(r, c, v) {
    for (let i = 0; i < n; i++) {
      if (i !== c && grid[r][i] === v) return false;
      if (i !== r && grid[i][c] === v) return false;
    }
    return true;
  }
  function cageOk(cgIdx) {
    const cg = cages[cgIdx];
    const vals = cg.cells.map(c => grid[(c / n) | 0][c % n]);
    const allFilled = vals.every(v => v);
    if (!allFilled) {
      const filled = vals.filter(v => v);
      if (cg.op === '+') {
        const sum = filled.reduce((a, b) => a + b, 0);
        if (sum > cg.target) return false;
      } else if (cg.op === '×') {
        let p = 1;
        for (const v of filled) p *= v;
        if (p > cg.target && cg.target > 0) return false;
      }
      return true;
    }
    if (cg.op === '=') return vals[0] === cg.target;
    if (cg.op === '+') return vals.reduce((a, b) => a + b, 0) === cg.target;
    if (cg.op === '×') return vals.reduce((a, b) => a * b, 1) === cg.target;
    if (cg.op === '-') return Math.abs(vals[0] - vals[1]) === cg.target;
    if (cg.op === '÷') {
      const [a, b] = vals;
      return (b > 0 && a / b === cg.target) || (a > 0 && b / a === cg.target);
    }
    return false;
  }
  function rec(idx) {
    if (count >= limit) return;
    if (idx === n * n) { count++; return; }
    const r = (idx / n) | 0, c = idx % n;
    for (let v = 1; v <= n; v++) {
      if (!valid(r, c, v)) continue;
      grid[r][c] = v;
      if (cageOk(cellCage[r * n + c])) rec(idx + 1);
      grid[r][c] = 0;
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
    const rng = seededRandom(seed + attempt * 1009);
    const sol = randomLatinSquare(n, rng);
    const regions = partitionIntoCages(n, rng, 4);
    // Reject puzzles with too many single-cell cages (too much given away).
    const singles = regions.filter(r => r.length === 1).length;
    if (singles > Math.max(2, n / 2)) continue;
    const cages = regions.map(reg => makeCage(reg, sol, n, rng));
    if (solveCount(n, cages, 2) === 1) {
      return { n, solution: sol, cages, levelIndex, cfg };
    }
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// `marks` is the live grid (n rows of n values 0..n; 0 = empty).
// Returns the set of cell indices currently in conflict:
//   * Row / column duplicates.
//   * Any cage whose all-filled values fail its op + target.
function findViolations(n, cages, marks) {
  const bad = new Set();
  for (let r = 0; r < n; r++) {
    const seen = new Map();
    for (let c = 0; c < n; c++) {
      const v = marks[r][c];
      if (!v) continue;
      if (seen.has(v)) { bad.add(r * n + c); bad.add(r * n + seen.get(v)); }
      else seen.set(v, c);
    }
  }
  for (let c = 0; c < n; c++) {
    const seen = new Map();
    for (let r = 0; r < n; r++) {
      const v = marks[r][c];
      if (!v) continue;
      if (seen.has(v)) { bad.add(r * n + c); bad.add(seen.get(v) * n + c); }
      else seen.set(v, r);
    }
  }
  for (const cg of cages) {
    const vals = cg.cells.map(c => marks[(c / n) | 0][c % n]);
    if (!vals.every(v => v)) continue;
    let ok = true;
    if (cg.op === '=') ok = vals[0] === cg.target;
    else if (cg.op === '+') ok = vals.reduce((a, b) => a + b, 0) === cg.target;
    else if (cg.op === '×') ok = vals.reduce((a, b) => a * b, 1) === cg.target;
    else if (cg.op === '-') ok = Math.abs(vals[0] - vals[1]) === cg.target;
    else if (cg.op === '÷') {
      const [a, b] = vals;
      ok = (b > 0 && a / b === cg.target) || (a > 0 && b / a === cg.target);
    }
    if (!ok) for (const c of cg.cells) bad.add(c);
  }
  return bad;
}

function isSolved(n, cages, marks) {
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (!marks[r][c]) return false;
  if (findViolations(n, cages, marks).size) return false;
  // Latin square exact check.
  for (let r = 0; r < n; r++) {
    const seen = new Set();
    for (let c = 0; c < n; c++) seen.add(marks[r][c]);
    if (seen.size !== n) return false;
  }
  for (let c = 0; c < n; c++) {
    const seen = new Set();
    for (let r = 0; r < n; r++) seen.add(marks[r][c]);
    if (seen.size !== n) return false;
  }
  return true;
}
