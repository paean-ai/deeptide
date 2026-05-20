// Pixel Hitori - shade cells so each row and column has no repeated unshaded
// number, no two shaded cells are orthogonally adjacent, and the unshaded
// cells form one connected region.
//
// Each level generates a random number grid from a seed, then a backtracking
// solver verifies the puzzle has exactly one valid shading.

const VW = 360, VH = 480;

// player cell states
const UNMARKED = 0, SHADED = 1, MARKED_OPEN = 2;

const LEVELS = [
  { name: ['Cottage', '小屋'], seed: 12,  n: 5 },
  { name: ['Garden', '花园'],  seed: 47,  n: 5 },
  { name: ['Manor', '庄园'],   seed: 98,  n: 6 },
  { name: ['Plaza', '广场'],   seed: 161, n: 6 },
  { name: ['Citadel', '城堡'], seed: 248, n: 7 },
  { name: ['Capital', '都城'], seed: 351, n: 7 },
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

const ix = (n, r, c) => r * n + c;

// Are all unshaded cells (sh[i]===0) connected as one region (4-adj)?
function unshadedConnected(sh, n) {
  let first = -1, totalUnshaded = 0;
  for (let i = 0; i < n * n; i++) if (sh[i] === 0) { totalUnshaded++; if (first < 0) first = i; }
  if (first < 0) return true;
  const seen = new Uint8Array(n * n);
  const stack = [first];
  seen[first] = 1;
  let count = 0;
  while (stack.length) {
    const cur = stack.pop();
    count++;
    const r = (cur / n) | 0, c = cur % n;
    for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
      const nr = r + dr, nc = c + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) continue;
      const ni = nr * n + nc;
      if (seen[ni] || sh[ni] !== 0) continue;
      seen[ni] = 1;
      stack.push(ni);
    }
  }
  return count === totalUnshaded;
}

// Backtracking solver. Returns { count, first } where first is the first valid
// shading as a Uint8Array, or null.
function solve(grid, n, limit) {
  const N = n * n;
  const sh = new Uint8Array(N);
  let found = 0, first = null;

  function adjShaded(i) {
    const r = (i / n) | 0, c = i % n;
    if (r > 0 && sh[i - n] === 1) return true;
    if (c > 0 && sh[i - 1] === 1) return true;
    return false;
  }
  function dupUnshaded(i, val) {
    const r = (i / n) | 0, c = i % n;
    for (let cc = 0; cc < c; cc++) if (sh[r * n + cc] === 0 && grid[r * n + cc] === val) return true;
    for (let rr = 0; rr < r; rr++) if (sh[rr * n + c] === 0 && grid[rr * n + c] === val) return true;
    return false;
  }
  function bt(i) {
    if (found >= limit) return;
    if (i === N) {
      if (unshadedConnected(sh, n)) {
        if (found === 0) first = sh.slice();
        found++;
      }
      return;
    }
    const v = grid[i];
    // try unshaded
    if (!dupUnshaded(i, v)) {
      sh[i] = 0;
      bt(i + 1);
    }
    if (found >= limit) { sh[i] = 0; return; }
    // try shaded
    if (!adjShaded(i)) {
      sh[i] = 1;
      bt(i + 1);
    }
    sh[i] = 0;
  }
  bt(0);
  return { count: found, first };
}

// Generate a random Latin square (each row and column is a permutation of
// 1..n).  Done by shifting + row/column permutation of the base shift-square.
function latinSquare(n, rng) {
  const base = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(((c + r) % n) + 1);
    base.push(row);
  }
  const rowOrder = base.map((_, i) => i);
  const colOrder = base.map((_, i) => i);
  for (let k = n - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [rowOrder[k], rowOrder[j]] = [rowOrder[j], rowOrder[k]];
  }
  for (let k = n - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [colOrder[k], colOrder[j]] = [colOrder[j], colOrder[k]];
  }
  const grid = new Uint8Array(n * n);
  for (let r = 0; r < n; r++)
    for (let c = 0; c < n; c++) grid[r * n + c] = base[rowOrder[r]][colOrder[c]];
  return grid;
}

// Pick `target` cells with no two orthogonally adjacent (greedy).
function pickShaded(n, rng, target) {
  const order = [];
  for (let i = 0; i < n * n; i++) order.push(i);
  for (let k = order.length - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [order[k], order[j]] = [order[j], order[k]];
  }
  const sh = new Uint8Array(n * n);
  let placed = 0;
  for (const i of order) {
    if (placed >= target) break;
    const r = (i / n) | 0, c = i % n;
    if (r > 0 && sh[i - n]) continue;
    if (r < n - 1 && sh[i + n]) continue;
    if (c > 0 && sh[i - 1]) continue;
    if (c < n - 1 && sh[i + 1]) continue;
    sh[i] = 1; placed++;
  }
  return sh;
}

// Build a uniquely-solvable Hitori puzzle: Latin square + sparse "duplicate"
// overwrites at shaded positions; verified by the solver.
function buildPuzzle(level) {
  const n = level.n, N = n * n;
  const rng = seededRandom(level.seed);
  const target = Math.max(2, Math.floor(N * 0.26));
  for (let attempt = 0; attempt < 800; attempt++) {
    const grid = latinSquare(n, rng);
    const sh = pickShaded(n, rng, target);
    // for each shaded cell, overwrite its value with another value from the
    // same row (creates an intra-row duplicate that the player must shade)
    for (let r = 0; r < n; r++) {
      for (let c = 0; c < n; c++) {
        if (!sh[r * n + c]) continue;
        let c2 = c;
        for (let g = 0; g < 8 && c2 === c; g++) c2 = (rng() * n) | 0;
        grid[r * n + c] = grid[r * n + c2];
      }
    }
    const res = solve(grid, n, 2);
    if (res.count === 1) return { n, grid, solution: res.first };
  }
  return null;
}

// ---- evaluation (player side) -------------------------------------------
// Build the live "violations" set for the current player cells (only SHADED
// counts as shaded; UNMARKED + MARKED_OPEN both count as unshaded).
function evaluate(pz, cells) {
  const n = pz.n, N = n * n;
  const bad = new Set();
  // shaded - no two orthogonal neighbours
  for (let i = 0; i < N; i++) {
    if (cells[i] !== SHADED) continue;
    const r = (i / n) | 0, c = i % n;
    for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
      const nr = r + dr, nc = c + dc;
      if (nr < 0 || nc < 0 || nr >= n || nc >= n) continue;
      const ni = nr * n + nc;
      if (cells[ni] === SHADED) { bad.add(i); bad.add(ni); }
    }
  }
  // unshaded - no duplicate values in any row / col
  for (let r = 0; r < n; r++) {
    const seenC = {};
    for (let c = 0; c < n; c++) {
      const i = r * n + c;
      if (cells[i] === SHADED) continue;
      const v = pz.grid[i];
      if (seenC[v] !== undefined) { bad.add(i); bad.add(seenC[v]); }
      else seenC[v] = i;
    }
  }
  for (let c = 0; c < n; c++) {
    const seenR = {};
    for (let r = 0; r < n; r++) {
      const i = r * n + c;
      if (cells[i] === SHADED) continue;
      const v = pz.grid[i];
      if (seenR[v] !== undefined) { bad.add(i); bad.add(seenR[v]); }
      else seenR[v] = i;
    }
  }
  // connectivity check (only matters at full assignment, but we report it)
  const sh = new Uint8Array(N);
  for (let i = 0; i < N; i++) sh[i] = (cells[i] === SHADED) ? 1 : 0;
  const conn = unshadedConnected(sh, n);
  const solved = bad.size === 0 && conn;
  return { bad, connected: conn, solved };
}

function cycleState(v) { return (v + 1) % 3; }
