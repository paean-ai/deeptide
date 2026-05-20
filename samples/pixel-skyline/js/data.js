// Pixel Skyline - a Skyscrapers visibility puzzle. Fill the grid so each row
// and column has the numbers 1..n exactly once; the clue at each side tells
// you how many "skyscrapers" you can see from that side.
//
// A taller building hides shorter buildings behind it - so for the row
// [3,1,4,2,5] viewed from the left you see 3,4,5 = 3 visible.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Outset', '初阵'],   seed: 13,  n: 4 },
  { name: ['District', '街区'], seed: 47,  n: 4 },
  { name: ['Quarter', '城区'],  seed: 96,  n: 5 },
  { name: ['Skyline', '天际线'], seed: 158, n: 5 },
  { name: ['Heart', '市心'],    seed: 241, n: 5 },
  { name: ['Capital', '都城'],  seed: 348, n: 5 },
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

// A random Latin square of size n: shift base + permute rows / columns.
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
  const out = [];
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(base[rowOrder[r]][colOrder[c]]);
    out.push(row);
  }
  return out;
}

// Number of buildings visible looking along a line of heights.
function visCount(line) {
  let max = 0, count = 0;
  for (const v of line) if (v > max) { max = v; count++; }
  return count;
}

// Derive the 4-sided clues from a Latin square solution.
function deriveClues(grid) {
  const n = grid.length;
  const top = [], bottom = [], left = [], right = [];
  for (let c = 0; c < n; c++) {
    const col = [];
    for (let r = 0; r < n; r++) col.push(grid[r][c]);
    top.push(visCount(col));
    bottom.push(visCount(col.slice().reverse()));
  }
  for (let r = 0; r < n; r++) {
    left.push(visCount(grid[r]));
    right.push(visCount(grid[r].slice().reverse()));
  }
  return { top, right, bottom, left };
}

// Count solutions matching the clues, stopping at `limit`. Captures the first.
function solve(n, clues, limit) {
  const grid = [];
  for (let r = 0; r < n; r++) grid.push(new Array(n).fill(0));
  let found = 0, first = null;

  function bt(r, c) {
    if (found >= limit) return;
    if (r === n) {
      // every column's top/bottom clue
      for (let col = 0; col < n; col++) {
        const colArr = [];
        for (let rr = 0; rr < n; rr++) colArr.push(grid[rr][col]);
        if (clues.top[col] && visCount(colArr) !== clues.top[col]) return;
        if (clues.bottom[col] && visCount(colArr.slice().reverse()) !== clues.bottom[col]) return;
      }
      if (found === 0) first = grid.map(row => row.slice());
      found++;
      return;
    }
    if (c === n) {
      // row done: check left/right clues
      if (clues.left[r] && visCount(grid[r]) !== clues.left[r]) return;
      if (clues.right[r] && visCount(grid[r].slice().reverse()) !== clues.right[r]) return;
      bt(r + 1, 0);
      return;
    }
    const rowUsed = new Set();
    for (let cc = 0; cc < c; cc++) rowUsed.add(grid[r][cc]);
    const colUsed = new Set();
    for (let rr = 0; rr < r; rr++) colUsed.add(grid[rr][c]);
    for (let v = 1; v <= n; v++) {
      if (rowUsed.has(v) || colUsed.has(v)) continue;
      grid[r][c] = v;
      bt(r, c + 1);
      grid[r][c] = 0;
      if (found >= limit) return;
    }
  }
  bt(0, 0);
  return { count: found, first };
}

function buildPuzzle(level) {
  const n = level.n;
  const rng = seededRandom(level.seed);
  for (let attempt = 0; attempt < 1000; attempt++) {
    const sol = latinSquare(n, rng);
    const clues = deriveClues(sol);
    const res = solve(n, clues, 2);
    if (res.count === 1) return { n, solution: sol, clues };
  }
  return null;
}

// ---- player evaluation --------------------------------------------------
function evaluate(pz, cells) {
  const n = pz.n;
  const bad = new Set();
  // row/col Latin constraint (among non-zero entries)
  for (let r = 0; r < n; r++) {
    const seen = {};
    for (let c = 0; c < n; c++) {
      const v = cells[r * n + c];
      if (!v) continue;
      if (seen[v] !== undefined) { bad.add(r * n + c); bad.add(seen[v]); }
      else seen[v] = r * n + c;
    }
  }
  for (let c = 0; c < n; c++) {
    const seen = {};
    for (let r = 0; r < n; r++) {
      const v = cells[r * n + c];
      if (!v) continue;
      if (seen[v] !== undefined) { bad.add(r * n + c); bad.add(seen[v]); }
      else seen[v] = r * n + c;
    }
  }
  // visibility clues (only validated on a fully-filled row/col)
  let solved = true;
  for (let r = 0; r < n; r++) {
    const row = [];
    for (let c = 0; c < n; c++) row.push(cells[r * n + c]);
    if (row.every(v => v > 0)) {
      if (pz.clues.left[r] && visCount(row) !== pz.clues.left[r]) solved = false;
      if (pz.clues.right[r] && visCount(row.slice().reverse()) !== pz.clues.right[r]) solved = false;
    } else solved = false;
  }
  for (let c = 0; c < n; c++) {
    const col = [];
    for (let r = 0; r < n; r++) col.push(cells[r * n + c]);
    if (col.every(v => v > 0)) {
      if (pz.clues.top[c] && visCount(col) !== pz.clues.top[c]) solved = false;
      if (pz.clues.bottom[c] && visCount(col.slice().reverse()) !== pz.clues.bottom[c]) solved = false;
    } else solved = false;
  }
  if (bad.size > 0) solved = false;
  return { bad, solved };
}
