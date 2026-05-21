// Pixel Numpath - a consecutive-number path puzzle.
//
// Fill the grid with 1 .. N so each number sits orthogonally next to the
// next - one snaking path that covers every cell. A handful of numbers are
// given. You build the path by tapping cell to cell from 1; a given number
// must be reached at exactly its step.
//
// Distinct from a king-move number-snake: every step here is orthogonal,
// so the answer is a true Hamiltonian path of the grid.
//
// Each level is { C, seed }. buildPuzzle is deterministic in the seed and
// returns { C, clues, levelIndex, cfg } where clues[i] is a given number
// (>=1) or 0 for a blank cell.

const VW = 360, VH = 480;

const LEVELS = [
  { name: ['Spark',   '星火'], C: 5, seed: 211 },
  { name: ['Ember',   '余烬'], C: 5, seed: 347 },
  { name: ['Vine',    '藤蔓'], C: 6, seed: 461 },
  { name: ['Maze',    '迷阵'], C: 6, seed: 577 },
  { name: ['Serpent', '长蛇'], C: 7, seed: 691 },
  { name: ['Odyssey', '远征'], C: 7, seed: 809 },
];
const LEVEL_COUNT = LEVELS.length;

const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

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

function adjacent(C, a, b) {
  return Math.abs((a % C) - (b % C)) + Math.abs(((a / C) | 0) - ((b / C) | 0)) === 1;
}

// ---- Hamiltonian path generation ---------------------------------------
// Start from the boustrophedon snake, then shuffle it with "backbite" moves
// (re-root an endpoint onto a random neighbour). Backbite preserves the
// Hamiltonian-path property, so generation can never fail.
function hamPath(C, rng) {
  const N = C * C;
  const path = [];
  for (let y = 0; y < C; y++) {
    if (y % 2 === 0) for (let x = 0; x < C; x++) path.push(y * C + x);
    else             for (let x = C - 1; x >= 0; x--) path.push(y * C + x);
  }
  const pos = new Array(N);
  for (let i = 0; i < N; i++) pos[path[i]] = i;
  const iters = N * 12;
  for (let it = 0; it < iters; it++) {
    const head = rng() < 0.5;
    const end = head ? path[0] : path[N - 1];
    const ex = end % C, ey = (end / C) | 0;
    const nbrs = [];
    for (const [dx, dy] of DIRS) {
      const nx = ex + dx, ny = ey + dy;
      if (nx >= 0 && nx < C && ny >= 0 && ny < C) nbrs.push(ny * C + nx);
    }
    const j = pos[nbrs[(rng() * nbrs.length) | 0]];
    if (head) {
      if (j < 2) continue;
      let lo = 0, hi = j - 1;
      while (lo < hi) { const t = path[lo]; path[lo] = path[hi]; path[hi] = t; lo++; hi--; }
      for (let i = 0; i < j; i++) pos[path[i]] = i;
    } else {
      if (j > N - 3) continue;
      let lo = j + 1, hi = N - 1;
      while (lo < hi) { const t = path[lo]; path[lo] = path[hi]; path[hi] = t; lo++; hi--; }
      for (let i = j + 1; i < N; i++) pos[path[i]] = i;
    }
  }
  return path;
}

// ---- uniqueness solver -------------------------------------------------
// Place 1, 2, 3 ... in order; each must be orthogonally next to the last.
// Tightly constrained, so this barely branches.
function countSolutions(C, clue, cap) {
  const N = C * C;
  const fixedCell = new Array(N + 1).fill(-1);
  for (let i = 0; i < N; i++) if (clue[i]) fixedCell[clue[i]] = i;
  const used = new Array(N).fill(false);
  const placed = new Array(N + 1).fill(-1);
  let count = 0;
  function rec(v) {
    if (count >= cap) return;
    if (v > N) { count++; return; }
    const prev = v > 1 ? placed[v - 1] : -1;
    let cands;
    if (fixedCell[v] >= 0) cands = [fixedCell[v]];
    else if (v === 1) { cands = []; for (let i = 0; i < N; i++) if (!used[i]) cands.push(i); }
    else {
      cands = [];
      const px = prev % C, py = (prev / C) | 0;
      for (const [dx, dy] of DIRS) {
        const nx = px + dx, ny = py + dy;
        if (nx >= 0 && nx < C && ny >= 0 && ny < C) {
          const ni = ny * C + nx;
          if (!used[ni]) cands.push(ni);
        }
      }
    }
    for (const c of cands) {
      if (used[c]) continue;
      if (prev >= 0 && !adjacent(C, prev, c)) continue;
      used[c] = true; placed[v] = c;
      rec(v + 1);
      used[c] = false;
      if (count >= cap) return;
    }
  }
  rec(1);
  return count;
}

// Drop given numbers at random while the puzzle stays uniquely solvable,
// keeping 1 and N as anchors and stopping at a floor.
function thinClues(C, full, rng, floor) {
  const work = full.slice();
  const N = C * C;
  let kept = N;
  const idx = [];
  for (let i = 0; i < N; i++) idx.push(i);
  shuffle(idx, rng);
  for (const i of idx) {
    if (kept <= floor) break;
    if (work[i] === 1 || work[i] === N) continue;
    const keep = work[i];
    work[i] = 0;
    if (countSolutions(C, work, 2) === 1) kept--;
    else work[i] = keep;
  }
  return work;
}

function buildPuzzle(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const C = cfg.C, N = C * C;
  const rng = seededRandom(cfg.seed);
  const path = hamPath(C, rng);
  const full = new Array(N);
  for (let k = 0; k < N; k++) full[path[k]] = k + 1;
  const clues = thinClues(C, full, rng, Math.round(N * 0.42));
  return { C, clues, levelIndex, cfg };
}

// ---- play helpers ------------------------------------------------------
// The player builds the path cell by cell from the "1" cell.
function startCell(puzzle) {
  for (let i = 0; i < puzzle.C * puzzle.C; i++) if (puzzle.clues[i] === 1) return i;
  return 0;
}

// Can the path be extended onto `cell`? It must be orthogonally adjacent to
// the path's end, unused, and - if it carries a given number - that number
// must equal the step about to be placed.
function canExtend(puzzle, path, cell) {
  if (!path.length) return false;
  if (path.indexOf(cell) !== -1) return false;
  if (!adjacent(puzzle.C, path[path.length - 1], cell)) return false;
  const given = puzzle.clues[cell];
  if (given && given !== path.length + 1) return false;
  return true;
}

function isSolved(puzzle, path) {
  return path.length === puzzle.C * puzzle.C;
}
