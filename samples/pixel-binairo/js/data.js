// Pixel Binairo - the binary logic puzzle (a.k.a. Takuzu / Unruly).
// Fill an n x n grid with 0s and 1s so that:
//   (1) no three of the same value sit consecutively in any row or column,
//   (2) every row and every column holds equal counts of 0 and 1,
//   (3) no two rows are identical, and no two columns are identical.
//
// Each level is generated from a seed: build a full valid grid, then
// carve clues away while a propagation + backtracking solver confirms
// the remaining clue set still forces a unique solution.

const VW = 360, VH = 480;
const EMPTY = -1;

const LEVELS = [
  { name: ['Spark',  '火花'], n: 6,  seed: 6101 },
  { name: ['Ember',  '余烬'], n: 6,  seed: 6202 },
  { name: ['Flame',  '焰火'], n: 8,  seed: 8303 },
  { name: ['Blaze',  '烈焰'], n: 8,  seed: 8404 },
  { name: ['Pyre',   '火堆'], n: 10, seed: 10505 },
  { name: ['Inferno','炼狱'], n: 10, seed: 10606 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- constraint checks -------------------------------------------------
// `grid` is a flat array length n*n; values 0, 1, or EMPTY.
function at(grid, n, r, c) { return grid[r * n + c]; }

// Would setting (r, c) = v keep the partial grid legal? (Checks only the
// constraints decidable from the cells already filled.)
function legalSet(grid, n, r, c, v) {
  const g = grid;
  const idx = r * n + c;
  const old = g[idx];
  g[idx] = v;
  let ok = true;
  // No three-in-a-row horizontally around (r, c).
  for (let cc = Math.max(0, c - 2); cc <= c && ok; cc++) {
    if (cc + 2 < n) {
      const a = g[r*n+cc], b = g[r*n+cc+1], d = g[r*n+cc+2];
      if (a !== EMPTY && a === b && b === d) ok = false;
    }
  }
  // Vertically.
  for (let rr = Math.max(0, r - 2); rr <= r && ok; rr++) {
    if (rr + 2 < n) {
      const a = g[rr*n+c], b = g[(rr+1)*n+c], d = g[(rr+2)*n+c];
      if (a !== EMPTY && a === b && b === d) ok = false;
    }
  }
  // Row count must not exceed n/2 of either value.
  if (ok) {
    let z = 0, o = 0;
    for (let cc = 0; cc < n; cc++) { const x = g[r*n+cc]; if (x===0) z++; else if (x===1) o++; }
    if (z > n/2 || o > n/2) ok = false;
  }
  // Column count.
  if (ok) {
    let z = 0, o = 0;
    for (let rr = 0; rr < n; rr++) { const x = g[rr*n+c]; if (x===0) z++; else if (x===1) o++; }
    if (z > n/2 || o > n/2) ok = false;
  }
  g[idx] = old;
  return ok;
}

// A complete grid is valid iff all three rule families hold.
function fullyValid(grid, n) {
  // No three-in-a-row.
  for (let r = 0; r < n; r++) for (let c = 0; c + 2 < n; c++) {
    if (grid[r*n+c] === grid[r*n+c+1] && grid[r*n+c+1] === grid[r*n+c+2]) return false;
  }
  for (let c = 0; c < n; c++) for (let r = 0; r + 2 < n; r++) {
    if (grid[r*n+c] === grid[(r+1)*n+c] && grid[(r+1)*n+c] === grid[(r+2)*n+c]) return false;
  }
  // Balanced rows / cols.
  for (let r = 0; r < n; r++) {
    let z = 0; for (let c = 0; c < n; c++) if (grid[r*n+c] === 0) z++;
    if (z !== n/2) return false;
  }
  for (let c = 0; c < n; c++) {
    let z = 0; for (let r = 0; r < n; r++) if (grid[r*n+c] === 0) z++;
    if (z !== n/2) return false;
  }
  // Unique rows + columns.
  const rowKeys = new Set(), colKeys = new Set();
  for (let r = 0; r < n; r++) {
    let k = '';
    for (let c = 0; c < n; c++) k += grid[r*n+c];
    if (rowKeys.has(k)) return false;
    rowKeys.add(k);
  }
  for (let c = 0; c < n; c++) {
    let k = '';
    for (let r = 0; r < n; r++) k += grid[r*n+c];
    if (colKeys.has(k)) return false;
    colKeys.add(k);
  }
  return true;
}

// ---- full-grid generator ----------------------------------------------
function buildFullGrid(n, rng) {
  const g = new Array(n * n).fill(EMPTY);
  function step(i) {
    if (i === n * n) return fullyValid(g, n);
    const r = (i / n) | 0, c = i % n;
    const order = rng() < 0.5 ? [0, 1] : [1, 0];
    for (const v of order) {
      if (legalSet(g, n, r, c, v)) {
        g[i] = v;
        if (step(i + 1)) return true;
        g[i] = EMPTY;
      }
    }
    return false;
  }
  return step(0) ? g : null;
}

// ---- uniqueness solver -------------------------------------------------
// Counts completions of a clue grid (capped at `cap`). Uses simple
// forced-move propagation before each branch.
function countSolutions(clue, n, cap = 2) {
  const g = clue.slice();
  let count = 0;
  function forced() {
    // Repeatedly fill any empty cell that has exactly one legal value.
    let changed = true;
    const filled = [];
    while (changed) {
      changed = false;
      for (let i = 0; i < n * n; i++) {
        if (g[i] !== EMPTY) continue;
        const r = (i / n) | 0, c = i % n;
        const c0 = legalSet(g, n, r, c, 0);
        const c1 = legalSet(g, n, r, c, 1);
        if (!c0 && !c1) { undo(filled); return false; }
        if (c0 !== c1) {
          g[i] = c0 ? 0 : 1;
          filled.push(i);
          changed = true;
        }
      }
    }
    return filled;
  }
  function undo(list) { for (const i of list) g[i] = EMPTY; }
  function dfs() {
    if (count >= cap) return;
    const filled = forced();
    if (filled === false) return;
    let pick = -1;
    for (let i = 0; i < n * n; i++) if (g[i] === EMPTY) { pick = i; break; }
    if (pick === -1) {
      if (fullyValid(g, n)) count++;
      undo(filled);
      return;
    }
    const r = (pick / n) | 0, c = pick % n;
    for (const v of [0, 1]) {
      if (legalSet(g, n, r, c, v)) {
        g[pick] = v;
        dfs();
        g[pick] = EMPTY;
        if (count >= cap) break;
      }
    }
    undo(filled);
  }
  dfs();
  return count;
}

// ---- puzzle build ------------------------------------------------------
function buildPuzzle(level) {
  const n = level.n;
  const rng = seededRandom(level.seed);
  for (let attempt = 0; attempt < 30; attempt++) {
    const full = buildFullGrid(n, rng);
    if (!full) continue;
    // Carve: hide cells in random order while uniqueness holds.
    const clue = full.slice();
    const order = [];
    for (let i = 0; i < n * n; i++) order.push(i);
    for (let i = order.length - 1; i > 0; i--) {
      const j = (rng() * (i + 1)) | 0;
      [order[i], order[j]] = [order[j], order[i]];
    }
    for (const i of order) {
      const saved = clue[i];
      clue[i] = EMPTY;
      if (countSolutions(clue, n, 2) !== 1) clue[i] = saved;
    }
    return { n, clue, solution: full };
  }
  return null;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const lv = LEVELS[levelIndex];
  const p = buildPuzzle(lv);
  if (!p) throw new Error('Binairo gen failed for level ' + levelIndex);
  const fixed = p.clue.map(v => v !== EMPTY);
  return {
    levelIndex, lv, n: p.n,
    clue: p.clue, fixed, solution: p.solution,
    grid: p.clue.slice(),
    elapsed: 0, solved: false,
  };
}

// Tap a non-fixed cell: EMPTY -> 0 -> 1 -> EMPTY.
function cycleCell(s, r, c) {
  if (s.solved) return false;
  const i = r * s.n + c;
  if (s.fixed[i]) return false;
  s.grid[i] = s.grid[i] === EMPTY ? 0 : s.grid[i] === 0 ? 1 : EMPTY;
  return true;
}

function isSolved(s) {
  for (let i = 0; i < s.n * s.n; i++) if (s.grid[i] === EMPTY) return false;
  return fullyValid(s.grid, s.n);
}

// Live conflicts: cells in a three-in-a-row run, or in an over-filled
// row/column. Returns a boolean grid.
function conflicts(s) {
  const n = s.n, g = s.grid;
  const bad = new Array(n * n).fill(false);
  // Three-in-a-row.
  for (let r = 0; r < n; r++) for (let c = 0; c + 2 < n; c++) {
    const a = g[r*n+c], b = g[r*n+c+1], d = g[r*n+c+2];
    if (a !== EMPTY && a === b && b === d) { bad[r*n+c] = bad[r*n+c+1] = bad[r*n+c+2] = true; }
  }
  for (let c = 0; c < n; c++) for (let r = 0; r + 2 < n; r++) {
    const a = g[r*n+c], b = g[(r+1)*n+c], d = g[(r+2)*n+c];
    if (a !== EMPTY && a === b && b === d) { bad[r*n+c] = bad[(r+1)*n+c] = bad[(r+2)*n+c] = true; }
  }
  // Over-filled rows / columns.
  for (let r = 0; r < n; r++) {
    let z = 0, o = 0;
    for (let c = 0; c < n; c++) { if (g[r*n+c]===0) z++; else if (g[r*n+c]===1) o++; }
    if (z > n/2 || o > n/2) for (let c = 0; c < n; c++) if (g[r*n+c] !== EMPTY) bad[r*n+c] = true;
  }
  for (let c = 0; c < n; c++) {
    let z = 0, o = 0;
    for (let r = 0; r < n; r++) { if (g[r*n+c]===0) z++; else if (g[r*n+c]===1) o++; }
    if (z > n/2 || o > n/2) for (let r = 0; r < n; r++) if (g[r*n+c] !== EMPTY) bad[r*n+c] = true;
  }
  return bad;
}
