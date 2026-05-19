// Pixel Dominosa - domino-partition puzzles: generation and the verifying solver.
//
// A grid is filled with pip numbers. The goal is to draw the boundaries of a
// full domino set over it - every domino (i,j) with 0 <= i <= j <= maxPip used
// exactly once. Each level lays a random domino set, writes the pips, then a
// backtracking solver confirms the pip grid has exactly ONE valid partition.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Sprout', '萌芽'],   seed: 14,  maxPip: 2 },
  { name: ['Garden', '花园'],   seed: 53,  maxPip: 3 },
  { name: ['Orchard', '果园'],  seed: 118, maxPip: 3 },
  { name: ['Grove', '林地'],    seed: 207, maxPip: 4 },
  { name: ['Thicket', '密林'],  seed: 326, maxPip: 4 },
  { name: ['Wildwood', '荒野'], seed: 472, maxPip: 5 },
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

function pairKey(a, b) { return a <= b ? a + ',' + b : b + ',' + a; }

// Count partitions of the pip grid into a full domino set, stopping at `limit`.
function solveCount(pips, rows, cols, limit) {
  const n = rows * cols;
  const covered = new Array(n).fill(false);
  const used = new Set();
  let found = 0;
  function bt() {
    if (found >= limit) return;
    let i = -1;
    for (let k = 0; k < n; k++) if (!covered[k]) { i = k; break; }
    if (i < 0) { found++; return; }
    const r = (i / cols) | 0, c = i % cols;
    const opts = [];
    if (c + 1 < cols && !covered[i + 1]) opts.push(i + 1);
    if (r + 1 < rows && !covered[i + cols]) opts.push(i + cols);
    for (const j of opts) {
      const key = pairKey(pips[i], pips[j]);
      if (used.has(key)) continue;
      used.add(key); covered[i] = covered[j] = true;
      bt();
      used.delete(key); covered[i] = covered[j] = false;
      if (found >= limit) return;
    }
  }
  bt();
  return found;
}

// A random domino tiling of the grid (row-major greedy; may deadlock -> null).
function randomTiling(rows, cols, rng) {
  const n = rows * cols;
  const covered = new Array(n).fill(false);
  const placements = [];
  for (let i = 0; i < n; i++) {
    if (covered[i]) continue;
    const r = (i / cols) | 0, c = i % cols;
    const nb = [];
    if (c + 1 < cols && !covered[i + 1]) nb.push(i + 1);
    if (r + 1 < rows && !covered[i + cols]) nb.push(i + cols);
    if (nb.length === 0) return null;
    const j = nb[(rng() * nb.length) | 0];
    covered[i] = covered[j] = true;
    placements.push([i, j]);
  }
  return placements;
}

// Build a uniquely-solvable Dominosa puzzle for a level from its seed.
function buildPuzzle(level) {
  const maxPip = level.maxPip;
  const rows = maxPip + 1, cols = maxPip + 2;
  const allPairs = [];
  for (let i = 0; i <= maxPip; i++) for (let j = i; j <= maxPip; j++) allPairs.push([i, j]);
  const rng = seededRandom(level.seed);

  for (let attempt = 0; attempt < 4000; attempt++) {
    const placements = randomTiling(rows, cols, rng);
    if (!placements || placements.length !== allPairs.length) continue;
    // assign each placement a distinct domino value-pair
    const pool = allPairs.slice();
    for (let k = pool.length - 1; k > 0; k--) {
      const j = (rng() * (k + 1)) | 0;
      [pool[k], pool[j]] = [pool[j], pool[k]];
    }
    const pips = new Array(rows * cols).fill(0);
    placements.forEach((pl, idx) => {
      const [v1, v2] = pool[idx];
      if (rng() < 0.5) { pips[pl[0]] = v1; pips[pl[1]] = v2; }
      else { pips[pl[0]] = v2; pips[pl[1]] = v1; }
    });
    if (solveCount(pips, rows, cols, 2) === 1) {
      return { rows, cols, maxPip, pips, solution: placements };
    }
  }
  return null;
}

// Evaluate the player's dominoes: covered cells, duplicate pairs, win.
function evaluate(pz, dominoes) {
  const covered = new Array(pz.rows * pz.cols).fill(false);
  const pairCount = {};
  const dup = new Set();
  dominoes.forEach((d, i) => {
    covered[d[0]] = covered[d[1]] = true;
    const key = pairKey(pz.pips[d[0]], pz.pips[d[1]]);
    (pairCount[key] || (pairCount[key] = [])).push(i);
  });
  for (const k in pairCount) if (pairCount[k].length > 1) pairCount[k].forEach(i => dup.add(i));
  const allCovered = covered.every(Boolean);
  return { covered, dup, solved: allCovered && dup.size === 0 };
}
