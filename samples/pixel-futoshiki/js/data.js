// Pixel Futoshiki - Latin square + inequality constraints.
//
// Rules:
//   * Fill every cell with a digit 1..n.
//   * Each row and each column contains every digit exactly once (Latin square).
//   * Between some pairs of orthogonally-adjacent cells there is a > or <
//     inequality sign — the cells must satisfy it.
//   * Find the unique completion.
//
// Each level is { n, seed }. buildPuzzle is deterministic per seed and
// returns { n, solution, clues (all-zero), constraints, levelIndex }.

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

// Random Latin square in 1..n by row shift + row/col/value permutation.
function randomLatinSquare(n, rng) {
  const base = new Array(n).fill(0).map((_, i) => i);
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

// Count solutions of (clues + constraints) up to `limit`. Most-constrained
// cell ordering would be better but simple row-major works at these sizes.
function solveCount(n, clues, constraints, limit) {
  const grid = clues.map(r => r.slice());
  const empties = [];
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    if (!grid[y][x]) empties.push([x, y]);
  }
  let count = 0;
  function valid(x, y, v) {
    for (let c = 0; c < n; c++) if (c !== x && grid[y][c] === v) return false;
    for (let r = 0; r < n; r++) if (r !== y && grid[r][x] === v) return false;
    return true;
  }
  function constraintsOk() {
    for (const cn of constraints) {
      const a = grid[cn.ay][cn.ax], b = grid[cn.by][cn.bx];
      if (!a || !b) continue;
      if (cn.op === '>' && !(a > b)) return false;
      if (cn.op === '<' && !(a < b)) return false;
    }
    return true;
  }
  function rec(idx) {
    if (count >= limit) return;
    if (idx === empties.length) {
      if (constraintsOk()) count++;
      return;
    }
    const [x, y] = empties[idx];
    for (let v = 1; v <= n; v++) {
      if (!valid(x, y, v)) continue;
      grid[y][x] = v;
      // Quick check: any constraint involving this cell whose other side is
      // already filled must hold.
      let ok = true;
      for (const cn of constraints) {
        if ((cn.ax === x && cn.ay === y) || (cn.bx === x && cn.by === y)) {
          const a = grid[cn.ay][cn.ax], b = grid[cn.by][cn.bx];
          if (a && b) {
            if (cn.op === '>' && !(a > b)) { ok = false; break; }
            if (cn.op === '<' && !(a < b)) { ok = false; break; }
          }
        }
      }
      if (ok) rec(idx + 1);
      grid[y][x] = 0;
      if (count >= limit) return;
    }
  }
  rec(0);
  return count;
}

// Pick a Latin square, build ALL > / < constraints between adjacent cells,
// then greedily REMOVE constraints while uniqueness still holds (no cell
// hints). The result is a pure-constraint Futoshiki.
function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const { n, seed } = cfg;
  for (let attempt = 0; attempt < 60; attempt++) {
    const rng = seededRandom(seed + attempt * 1009);
    const solution = randomLatinSquare(n, rng);
    const allPairs = [];
    for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
      if (x + 1 < n) allPairs.push([x, y, x + 1, y]);
      if (y + 1 < n) allPairs.push([x, y, x, y + 1]);
    }
    for (let i = allPairs.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      [allPairs[i], allPairs[j]] = [allPairs[j], allPairs[i]];
    }
    const allConstraints = allPairs.map(([ax, ay, bx, by]) => {
      const va = solution[ay][ax], vb = solution[by][bx];
      return { ax, ay, bx, by, op: va > vb ? '>' : '<' };
    });
    const empty = new Array(n).fill(0).map(() => new Array(n).fill(0));
    // NOTE: even with EVERY adjacent inequality constraint, the solver
    // sometimes finds multiple Latin squares satisfying all of them (small
    // grids have just enough symmetries for a sign-equivalent square to slip
    // through). Reject those attempts and retry with a fresh Latin square.
    if (solveCount(n, empty, allConstraints, 2) !== 1) continue;
    // Trim constraints greedily while uniqueness holds.
    const kept = allConstraints.slice();
    const order = kept.map((_, i) => i);
    for (let i = order.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      [order[i], order[j]] = [order[j], order[i]];
    }
    for (const i of order) {
      const saved = kept[i];
      kept[i] = null;
      const trimmed = kept.filter(c => c);
      if (solveCount(n, empty, trimmed, 2) !== 1) kept[i] = saved;
    }
    const finalConstraints = kept.filter(c => c);
    return { n, solution, clues: empty, constraints: finalConstraints, levelIndex, cfg };
  }
  return null;
}

// ---- live validation (for UI red-flag) ---------------------------------
// `marks` is the live grid (rows of values 0..n; 0 = empty). Returns the set
// of cell indices currently in conflict.
function findViolations(n, constraints, marks) {
  const bad = new Set();
  // Row + col duplicates.
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
  // Constraint breaks.
  for (const cn of constraints) {
    const a = marks[cn.ay][cn.ax], b = marks[cn.by][cn.bx];
    if (!a || !b) continue;
    if ((cn.op === '>' && !(a > b)) || (cn.op === '<' && !(a < b))) {
      bad.add(cn.ay * n + cn.ax);
      bad.add(cn.by * n + cn.bx);
    }
  }
  return bad;
}

function isSolved(n, constraints, marks) {
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (!marks[r][c]) return false;
  if (findViolations(n, constraints, marks).size) return false;
  // Latin square: every row contains 1..n.
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
