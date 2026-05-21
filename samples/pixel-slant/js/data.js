// Pixel Slant - the classic Slant (Gokigen Naname) puzzle.
//
// Draw a diagonal in every cell - either '\' or '/'. A numbered lattice
// point must be touched by exactly that many diagonals, and the diagonals
// must never close a loop.
//
// Each level is { C, seed }. buildPuzzle is deterministic in the seed and
// returns { C, clues, levelIndex, cfg } where clues[v] is the lattice-point
// clue (0..4) or -1 for an unnumbered point.

const VW = 360, VH = 480;

// Cell diagonal codes.
const D_NONE = 0, D_BACK = 1, D_FWD = 2;   // '', '\', '/'

const LEVELS = [
  { name: ['Sketch',  '草图'], C: 5, seed: 211 },
  { name: ['Hatch',   '斜线'], C: 5, seed: 331 },
  { name: ['Mesh',    '网格'], C: 6, seed: 443 },
  { name: ['Weave',   '交织'], C: 6, seed: 557 },
  { name: ['Skein',   '缠结'], C: 7, seed: 661 },
  { name: ['Labyrinth', '迷宫'], C: 7, seed: 787 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}
function shuffle(a, rng) {
  for (let i = a.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    const t = a[i]; a[i] = a[j]; a[j] = t;
  }
  return a;
}

// Lattice-point id; the grid has (C+1)x(C+1) points.
function vid(C, r, c) { return r * (C + 1) + c; }

// The two lattice points a cell's diagonal connects.
//   '\' joins the top-left and bottom-right points.
//   '/' joins the top-right and bottom-left points.
function diagEnds(C, cell, diag) {
  const r = (cell / C) | 0, c = cell % C;
  if (diag === D_BACK) return [vid(C, r, c), vid(C, r + 1, c + 1)];
  return [vid(C, r, c + 1), vid(C, r + 1, c)];
}

// ---- generation: a random loop-free full solution ----------------------
// Lay a diagonal in each cell in random order; a cell takes whichever
// diagonal does not close a loop. The diagonals form a forest, so a valid
// loop-free solution always exists.
function genSolution(C, rng) {
  const nV = (C + 1) * (C + 1), N = C * C;
  for (let attempt = 0; attempt < 80; attempt++) {
    const parent = [];
    for (let i = 0; i < nV; i++) parent[i] = i;
    const find = x => { while (parent[x] !== x) x = parent[x]; return x; };
    const order = shuffle(Array.from({ length: N }, (_, i) => i), rng);
    const sol = new Array(N);
    let ok = true;
    for (const cell of order) {
      const a = diagEnds(C, cell, D_BACK), b = diagEnds(C, cell, D_FWD);
      const aOk = find(a[0]) !== find(a[1]);
      const bOk = find(b[0]) !== find(b[1]);
      let diag;
      if (aOk && bOk) diag = rng() < 0.5 ? D_BACK : D_FWD;
      else if (aOk)   diag = D_BACK;
      else if (bOk)   diag = D_FWD;
      else { ok = false; break; }
      const e = diagEnds(C, cell, diag);
      parent[find(e[0])] = find(e[1]);
      sol[cell] = diag;
    }
    if (ok) return sol;
  }
  return null;
}

function fullClues(C, sol) {
  const cl = new Array((C + 1) * (C + 1)).fill(0);
  for (let cell = 0; cell < C * C; cell++) {
    const e = diagEnds(C, cell, sol[cell]);
    cl[e[0]]++; cl[e[1]]++;
  }
  return cl;
}

// ---- uniqueness solver -------------------------------------------------
// Backtrack cell by cell trying each diagonal; prune on lattice-point
// over/under counts and on any loop. Count solutions up to `cap`.
function countSolutions(C, clue, cap) {
  const N = C * C, nV = (C + 1) * (C + 1);
  const vCount = new Array(nV).fill(0);
  const vRemain = new Array(nV).fill(0);
  const cellTouch = [];
  for (let cell = 0; cell < N; cell++) {
    const set = new Set();
    for (const v of diagEnds(C, cell, D_BACK)) set.add(v);
    for (const v of diagEnds(C, cell, D_FWD)) set.add(v);
    const arr = [...set];
    cellTouch.push(arr);
    for (const v of arr) vRemain[v]++;
  }
  const parent = [];
  for (let i = 0; i < nV; i++) parent[i] = i;
  const find = x => { while (parent[x] !== x) x = parent[x]; return x; };
  let count = 0;
  function rec(cell) {
    if (count >= cap) return;
    if (cell === N) {
      for (let v = 0; v < nV; v++) if (clue[v] >= 0 && vCount[v] !== clue[v]) return;
      count++;
      return;
    }
    const touch = cellTouch[cell];
    for (const v of touch) vRemain[v]--;
    for (let diag = D_BACK; diag <= D_FWD; diag++) {
      const e = diagEnds(C, cell, diag);
      const ra = find(e[0]), rb = find(e[1]);
      if (ra === rb) continue;                       // would close a loop
      vCount[e[0]]++; vCount[e[1]]++;
      let ok = true;
      for (const v of e) {
        if (clue[v] >= 0 && (vCount[v] > clue[v] || vCount[v] + vRemain[v] < clue[v])) { ok = false; break; }
      }
      if (ok) for (const v of touch) {
        if (v === e[0] || v === e[1]) continue;
        if (clue[v] >= 0 && vCount[v] + vRemain[v] < clue[v]) { ok = false; break; }
      }
      if (ok) {
        parent[rb] = ra;
        rec(cell + 1);
        parent[rb] = rb;
      }
      vCount[e[0]]--; vCount[e[1]]--;
      if (count >= cap) break;
    }
    for (const v of touch) vRemain[v]++;
  }
  rec(0);
  return count;
}

// Drop clues at random while the puzzle stays uniquely solvable, keeping a
// floor so the board never gets so sparse that solving slows down.
function thinClues(C, full, rng, floor) {
  const work = full.slice();
  let kept = work.length;
  const idx = shuffle(Array.from({ length: work.length }, (_, i) => i), rng);
  for (const i of idx) {
    if (kept <= floor) break;
    const keep = work[i];
    work[i] = -1;
    if (countSolutions(C, work, 2) === 1) kept--;
    else work[i] = keep;
  }
  return work;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const C = cfg.C;
  const rng = seededRandom(cfg.seed);
  const sol = genSolution(C, rng);
  const full = fullClues(C, sol);
  const clues = thinClues(C, full, rng, Math.round((C + 1) * (C + 1) * 0.5));
  return { C, clues, levelIndex, cfg };
}

// ---- live state --------------------------------------------------------
// Per lattice-point: how many drawn diagonals touch it, and whether that
// already breaks its clue (too many).
function pointState(C, clues, cells) {
  const nV = (C + 1) * (C + 1);
  const count = new Array(nV).fill(0);
  for (let cell = 0; cell < C * C; cell++) {
    if (!cells[cell]) continue;
    const e = diagEnds(C, cell, cells[cell]);
    count[e[0]]++; count[e[1]]++;
  }
  return count;
}

// Cells whose diagonal lies on a loop - the 2-core of the drawn graph.
function loopCells(C, cells) {
  const nV = (C + 1) * (C + 1);
  const deg = new Array(nV).fill(0);
  const drawn = [];
  for (let cell = 0; cell < C * C; cell++) {
    if (!cells[cell]) continue;
    const e = diagEnds(C, cell, cells[cell]);
    deg[e[0]]++; deg[e[1]]++;
    drawn.push({ cell, e });
  }
  // Peel leaves (degree <= 1) repeatedly; survivors lie on a loop.
  const alive = drawn.map(() => true);
  let changed = true;
  while (changed) {
    changed = false;
    for (let i = 0; i < drawn.length; i++) {
      if (!alive[i]) continue;
      const [u, v] = drawn[i].e;
      if (deg[u] <= 1 || deg[v] <= 1) {
        alive[i] = false;
        deg[u]--; deg[v]--;
        changed = true;
      }
    }
  }
  const set = new Set();
  for (let i = 0; i < drawn.length; i++) if (alive[i]) set.add(drawn[i].cell);
  return set;
}

function isSolved(C, clues, cells) {
  for (let cell = 0; cell < C * C; cell++) if (!cells[cell]) return false;   // every cell filled
  if (loopCells(C, cells).size) return false;                               // no loops
  const count = pointState(C, clues, cells);
  for (let v = 0; v < count.length; v++) if (clues[v] >= 0 && count[v] !== clues[v]) return false;
  return true;
}
