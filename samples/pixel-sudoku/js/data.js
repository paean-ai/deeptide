// Pixel Sudoku - level definitions and puzzle generation.

const VW = 360, VH = 480;

// tier 0 Easy, 1 Medium, 2 Hard. holes = target cells to dig out (capped by
// the uniqueness constraint, so a puzzle may keep a few more givens).
const LEVELS = [
  { tier: 0, seed: 1841, holes: 38 },
  { tier: 0, seed: 5093, holes: 41 },
  { tier: 0, seed: 12604, holes: 43 },
  { tier: 0, seed: 20118, holes: 44 },
  { tier: 0, seed: 27905, holes: 45 },
  { tier: 0, seed: 35462, holes: 45 },
  { tier: 1, seed: 2677, holes: 46 },
  { tier: 1, seed: 8214, holes: 49 },
  { tier: 1, seed: 15330, holes: 50 },
  { tier: 1, seed: 22904, holes: 51 },
  { tier: 1, seed: 30471, holes: 51 },
  { tier: 1, seed: 38108, holes: 52 },
  { tier: 2, seed: 4408, holes: 52 },
  { tier: 2, seed: 9526, holes: 54 },
  { tier: 2, seed: 17782, holes: 55 },
  { tier: 2, seed: 25617, holes: 56 },
  { tier: 2, seed: 33188, holes: 57 },
  { tier: 2, seed: 40926, holes: 57 },
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
function shuffled(arr, rng) {
  const a = arr.slice();
  for (let i = a.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

// Can digit v be placed at cell i in grid g?
function sudokuValid(g, i, v) {
  const r = (i / 9) | 0, c = i % 9;
  for (let k = 0; k < 9; k++) {
    if (g[r * 9 + k] === v) return false;
    if (g[k * 9 + c] === v) return false;
  }
  const br = Math.floor(r / 3) * 3, bc = Math.floor(c / 3) * 3;
  for (let dr = 0; dr < 3; dr++) {
    for (let dc = 0; dc < 3; dc++) {
      if (g[(br + dr) * 9 + (bc + dc)] === v) return false;
    }
  }
  return true;
}

// A complete, valid, randomised solution grid.
function genSolution(rng) {
  const g = new Array(81).fill(0);
  function fill(i) {
    if (i === 81) return true;
    for (const v of shuffled([1, 2, 3, 4, 5, 6, 7, 8, 9], rng)) {
      if (sudokuValid(g, i, v)) {
        g[i] = v;
        if (fill(i + 1)) return true;
        g[i] = 0;
      }
    }
    return false;
  }
  fill(0);
  return g;
}

// Count solutions up to `limit`, choosing the most-constrained cell (fast).
function solveCount(grid, limit) {
  const g = grid.slice();
  let count = 0;
  function rec() {
    if (count >= limit) return;
    let best = -1, bestCand = null;
    for (let i = 0; i < 81; i++) {
      if (g[i] !== 0) continue;
      const cand = [];
      for (let v = 1; v <= 9; v++) if (sudokuValid(g, i, v)) cand.push(v);
      if (cand.length === 0) return;
      if (best < 0 || cand.length < bestCand.length) {
        best = i; bestCand = cand;
        if (cand.length === 1) break;
      }
    }
    if (best < 0) { count++; return; }
    for (const v of bestCand) {
      g[best] = v;
      rec();
      g[best] = 0;
      if (count >= limit) return;
    }
  }
  rec();
  return count;
}

// Remove cells while the puzzle keeps exactly one solution.
function generatePuzzle(seed, targetHoles) {
  const rng = seededRandom(seed);
  const solution = genSolution(rng);
  const puzzle = solution.slice();
  let holes = 0;
  for (const i of shuffled([...Array(81).keys()], rng)) {
    if (holes >= targetHoles) break;
    const saved = puzzle[i];
    puzzle[i] = 0;
    if (solveCount(puzzle, 2) !== 1) puzzle[i] = saved;
    else holes++;
  }
  return { puzzle, solution, holes };
}
