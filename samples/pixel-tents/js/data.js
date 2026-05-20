// Pixel Tents - the Tents and Trees logic puzzle. Place a tent next to every
// tree (one tent per tree, one tree per tent), tents may not touch even
// diagonally, and the row / column counts must match.
//
// Each level seeds a random pairing of trees and tents; a backtracking solver
// verifies the row / column count clues have exactly one solution.

const VW = 360, VH = 480;

// player cell states
const EMPTY = 0, TENT = 1, GRASS = 2;
// fixed cell types
const F_EMPTY = 0, F_TREE = 1;

const LEVELS = [
  { name: ['Meadow', '草甸'], seed: 14,  n: 6 },
  { name: ['Grove', '林地'],  seed: 47,  n: 6 },
  { name: ['Glade', '林间空地'], seed: 95, n: 7 },
  { name: ['Forest', '森林'], seed: 158, n: 7 },
  { name: ['Wildwood', '荒林'], seed: 241, n: 8 },
  { name: ['Old Wood', '老林'], seed: 348, n: 8 },
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
const ORTH = [[-1, 0], [1, 0], [0, -1], [0, 1]];
const NEIGH8 = [[-1, -1], [-1, 0], [-1, 1], [0, -1], [0, 1], [1, -1], [1, 0], [1, 1]];

function inBounds(n, r, c) { return r >= 0 && c >= 0 && r < n && c < n; }

// ---- generation ---------------------------------------------------------
// Place K non-adjacent (incl. diag) tents at random; for each, pick an orth-
// empty neighbour as its tree. Returns { fixed (1=tree), solution (1=tent) }
// or null if it can't satisfy the layout.
function placePairs(n, target, rng) {
  const fixed = new Uint8Array(n * n);    // 0 empty, 1 tree
  const sol = new Uint8Array(n * n);       // 0 empty, 1 tent
  const order = [];
  for (let i = 0; i < n * n; i++) order.push(i);
  for (let k = order.length - 1; k > 0; k--) {
    const j = (rng() * (k + 1)) | 0;
    [order[k], order[j]] = [order[j], order[k]];
  }
  let placed = 0;
  for (const i of order) {
    if (placed >= target) break;
    const r = (i / n) | 0, c = i % n;
    if (sol[i] || fixed[i]) continue;
    // no adjacent tent (8-neigh) and not adjacent to an existing tree at this cell
    let adjTent = false;
    for (const [dr, dc] of NEIGH8) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (sol[ix(n, nr, nc)]) { adjTent = true; break; }
    }
    if (adjTent) continue;
    // pick an orthog-empty neighbour as the tree
    const opts = [];
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      const j = ix(n, nr, nc);
      if (!sol[j] && !fixed[j]) opts.push(j);
    }
    if (!opts.length) continue;
    sol[i] = 1;
    fixed[opts[(rng() * opts.length) | 0]] = 1;
    placed++;
  }
  if (placed < target) return null;
  return { fixed, sol, placed };
}

function rowColCounts(sol, n) {
  const row = new Array(n).fill(0), col = new Array(n).fill(0);
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (sol[ix(n, r, c)]) {
    row[r]++; col[c]++;
  }
  return { row, col };
}

// ---- solver --------------------------------------------------------------
// Count solutions for a Tents puzzle (fixed trees + row/col counts).
function solveCount(n, fixed, rowCnt, colCnt, limit) {
  const N = n * n;
  const trees = [];
  for (let i = 0; i < N; i++) if (fixed[i]) trees.push(i);
  // candidate tent cells per tree: orthog neighbours that are not trees
  const cand = trees.map(t => {
    const r = (t / n) | 0, c = t % n;
    const out = [];
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      const j = ix(n, nr, nc);
      if (!fixed[j]) out.push(j);
    }
    return out;
  });
  const tent = new Uint8Array(N);
  const rowUsed = new Array(n).fill(0);
  const colUsed = new Array(n).fill(0);
  const treeMatched = new Uint8Array(trees.length);   // 1 = paired
  let found = 0;

  // a candidate cell is valid if no adjacent (8-neigh) cell is already a tent
  function canPlace(j) {
    if (tent[j]) return false;
    const r = (j / n) | 0, c = j % n;
    if (rowUsed[r] >= rowCnt[r]) return false;
    if (colUsed[c] >= colCnt[c]) return false;
    for (const [dr, dc] of NEIGH8) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (tent[ix(n, nr, nc)]) return false;
    }
    return true;
  }
  function place(j) {
    tent[j] = 1;
    rowUsed[(j / n) | 0]++; colUsed[j % n]++;
  }
  function unplace(j) {
    tent[j] = 0;
    rowUsed[(j / n) | 0]--; colUsed[j % n]--;
  }

  // process trees in order of fewest candidates first (most-constrained first)
  const order = trees.map((_, i) => i)
    .sort((a, b) => cand[a].length - cand[b].length);

  function bt(k) {
    if (found >= limit) return;
    if (k === trees.length) {
      // every tree paired, and every tent must have at least one adjacent tree
      // (since we only ever placed tents at tree-adjacent candidates, this holds)
      // verify counts exactly match
      for (let i = 0; i < n; i++) if (rowUsed[i] !== rowCnt[i] || colUsed[i] !== colCnt[i]) return;
      found++;
      return;
    }
    const ti = order[k];
    for (const j of cand[ti]) {
      if (!canPlace(j)) continue;
      // skip if this candidate would be "stolen" by another later tree exclusively?
      // (not enforced - allow over-placement, count check at end catches it)
      place(j);
      treeMatched[ti] = 1;
      bt(k + 1);
      unplace(j);
      treeMatched[ti] = 0;
      if (found >= limit) return;
    }
  }
  bt(0);
  return found;
}

// Build a uniquely-solvable Tents puzzle from a seed.
function buildPuzzle(level) {
  const n = level.n;
  const target = Math.max(3, Math.floor(n * n * 0.22));
  const rng = seededRandom(level.seed);
  for (let attempt = 0; attempt < 1200; attempt++) {
    const p = placePairs(n, target, rng);
    if (!p) continue;
    const { row, col } = rowColCounts(p.sol, n);
    if (solveCount(n, p.fixed, row, col, 2) === 1) {
      return { n, fixed: p.fixed, solution: p.sol, rowCnt: row, colCnt: col };
    }
  }
  return null;
}

// ---- evaluation (game side) ---------------------------------------------
function evaluate(pz, cells) {
  const n = pz.n;
  const bad = new Set();
  const rowUsed = new Array(n).fill(0);
  const colUsed = new Array(n).fill(0);
  // tents: no two adjacent (incl diag), and row/col counts
  for (let i = 0; i < n * n; i++) {
    if (cells[i] !== TENT) continue;
    const r = (i / n) | 0, c = i % n;
    rowUsed[r]++; colUsed[c]++;
    for (const [dr, dc] of NEIGH8) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (cells[ix(n, nr, nc)] === TENT) { bad.add(i); bad.add(ix(n, nr, nc)); }
    }
  }
  // every tent should have a tree neighbour
  for (let i = 0; i < n * n; i++) {
    if (cells[i] !== TENT) continue;
    const r = (i / n) | 0, c = i % n;
    let hasTree = false;
    for (const [dr, dc] of ORTH) {
      const nr = r + dr, nc = c + dc;
      if (!inBounds(n, nr, nc)) continue;
      if (pz.fixed[ix(n, nr, nc)]) { hasTree = true; break; }
    }
    if (!hasTree) bad.add(i);
  }
  let solved = true;
  for (let r = 0; r < n; r++) if (rowUsed[r] !== pz.rowCnt[r]) solved = false;
  for (let c = 0; c < n; c++) if (colUsed[c] !== pz.colCnt[c]) solved = false;
  if (bad.size > 0) solved = false;
  // each tree must be adjacent to >= 1 tent
  if (solved) {
    for (let i = 0; i < n * n; i++) {
      if (!pz.fixed[i]) continue;
      const r = (i / n) | 0, c = i % n;
      let hasTent = false;
      for (const [dr, dc] of ORTH) {
        const nr = r + dr, nc = c + dc;
        if (!inBounds(n, nr, nc)) continue;
        if (cells[ix(n, nr, nc)] === TENT) { hasTent = true; break; }
      }
      if (!hasTent) { solved = false; break; }
    }
  }
  return { bad, solved, rowUsed, colUsed };
}

function cycleCell(v) { return (v + 1) % 3; }
