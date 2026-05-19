// Pixel Glyphs - a Lights Out puzzle: pressing a glyph flips it and its four
// neighbours; light every glyph to break the lock.
//
// Each level is scrambled by pressing a random set of glyphs from the solved
// (all-lit) board, so it is always solvable by construction. A GF(2) solver
// then finds the minimum-press solution, used as the level's par.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Seal', '封印'],    seed: 21,  n: 3 },
  { name: ['Ward', '守印'],    seed: 64,  n: 4 },
  { name: ['Sigil', '符印'],   seed: 138, n: 4 },
  { name: ['Cipher', '密印'],  seed: 255, n: 5 },
  { name: ['Lattice', '阵图'], seed: 392, n: 5 },
  { name: ['Vault', '秘库'],   seed: 540, n: 6 },
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

// cells toggled by pressing (r,c): itself and orthogonal neighbours
function pressCells(n, r, c) {
  const out = [r * n + c];
  if (r > 0) out.push((r - 1) * n + c);
  if (r < n - 1) out.push((r + 1) * n + c);
  if (c > 0) out.push(r * n + c - 1);
  if (c < n - 1) out.push(r * n + c + 1);
  return out;
}

// apply a press to a 0/1 grid (1 = lit), mutating it
function applyPress(grid, n, r, c) {
  for (const i of pressCells(n, r, c)) grid[i] ^= 1;
}

function isLit(grid) {
  for (let i = 0; i < grid.length; i++) if (!grid[i]) return false;
  return true;
}

// Minimum number of presses to light every glyph, via GF(2) elimination.
// Returns the press count, or -1 if the board is unsolvable.
function solveMin(state, n) {
  const N = n * n;
  const rows = [];
  for (let j = 0; j < N; j++) {
    const row = new Uint8Array(N + 1);
    for (const i of pressCells(n, (j / n) | 0, j % n)) row[i] = 1;
    row[N] = 1 ^ state[j];
    rows.push(row);
  }
  const pivotCol = [];
  let rank = 0;
  for (let c = 0; c < N && rank < N; c++) {
    let sel = -1;
    for (let k = rank; k < N; k++) if (rows[k][c]) { sel = k; break; }
    if (sel < 0) continue;
    [rows[rank], rows[sel]] = [rows[sel], rows[rank]];
    for (let k = 0; k < N; k++) {
      if (k !== rank && rows[k][c]) {
        for (let x = 0; x <= N; x++) rows[k][x] ^= rows[rank][x];
      }
    }
    pivotCol[rank] = c;
    rank++;
  }
  for (let k = rank; k < N; k++) if (rows[k][N]) return -1;
  const isPivot = new Array(N).fill(false);
  for (let k = 0; k < rank; k++) isPivot[pivotCol[k]] = true;
  const free = [];
  for (let c = 0; c < N; c++) if (!isPivot[c]) free.push(c);
  if (free.length > 22) return null;
  let best = Infinity;
  for (let mask = 0; mask < (1 << free.length); mask++) {
    const x = new Uint8Array(N);
    for (let i = 0; i < free.length; i++) if (mask & (1 << i)) x[free[i]] = 1;
    for (let k = rank - 1; k >= 0; k--) {
      let v = rows[k][N];
      for (let c = pivotCol[k] + 1; c < N; c++) if (rows[k][c] && x[c]) v ^= 1;
      x[pivotCol[k]] = v;
    }
    let w = 0;
    for (let i = 0; i < N; i++) w += x[i];
    if (w < best) best = w;
  }
  return best;
}

// Build a level: scramble the solved board, then compute its par.
function buildPuzzle(level) {
  const n = level.n, N = n * n;
  const rng = seededRandom(level.seed);
  let grid;
  for (let attempt = 0; attempt < 200; attempt++) {
    grid = new Uint8Array(N).fill(1);
    let presses = 0;
    for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) {
      if (rng() < 0.5) { applyPress(grid, n, r, c); presses++; }
    }
    if (presses > 0 && !isLit(grid)) break;
  }
  return { n, start: Array.from(grid), par: solveMin(grid, n) };
}
