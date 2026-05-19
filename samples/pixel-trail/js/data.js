// Pixel Trail - a Hidato-style number-snake puzzle. Numbers 1..N form a path
// through every cell where consecutive numbers sit on orthogonal neighbours.
//
// Each level builds a random Hamiltonian path of the grid, numbers the cells
// 1..N along it, and reveals just enough numbers as clues for the puzzle to
// have exactly one valid solution (verified by a backtracking solver).

const VW = 360, VH = 480;
const DIRS = [[-1, 0], [1, 0], [0, -1], [0, 1]];

const LEVELS = [
  { name: ['Path', '小径'],     seed: 21,  n: 5 },
  { name: ['Loop', '回环'],     seed: 67,  n: 5 },
  { name: ['Garden', '花园'],   seed: 134, n: 6 },
  { name: ['Maze', '迷宫'],     seed: 218, n: 6 },
  { name: ['Spiral', '螺旋'],   seed: 329, n: 6 },
  { name: ['Labyrinth', '迷阵'], seed: 467, n: 6 },
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

function neighbors(n, idx) {
  const r = (idx / n) | 0, c = idx % n;
  const out = [];
  for (const [dr, dc] of DIRS) {
    const nr = r + dr, nc = c + dc;
    if (nr >= 0 && nc >= 0 && nr < n && nc < n) out.push(nr * n + nc);
  }
  return out;
}

// Build a random Hamiltonian path on the n x n grid via shuffled DFS.
function findHamilton(n, rng) {
  const N = n * n;
  const seen = new Array(N);
  const path = [];
  function dfs(cell, depth) {
    if (depth >= 600 * N) return false; // node cap
    seen[cell] = true;
    path.push(cell);
    if (path.length === N) return true;
    const nbs = neighbors(n, cell).slice();
    for (let k = nbs.length - 1; k > 0; k--) {
      const j = (rng() * (k + 1)) | 0;
      [nbs[k], nbs[j]] = [nbs[j], nbs[k]];
    }
    for (const nb of nbs) {
      if (!seen[nb] && dfs(nb, depth + 1)) return true;
    }
    seen[cell] = false;
    path.pop();
    return false;
  }
  for (let tries = 0; tries < 80; tries++) {
    for (let i = 0; i < N; i++) seen[i] = false;
    path.length = 0;
    if (dfs((rng() * N) | 0, 0)) return path.slice();
  }
  return null;
}

// Count Hidato solutions consistent with the clue map (k -> cell), to limit.
function solveCount(n, N, clueMap, limit) {
  const cellOfClueK = clueMap;                 // k -> cell
  const clueAtCell = {};                       // cell -> k
  for (const k in cellOfClueK) clueAtCell[cellOfClueK[k]] = +k;
  const used = new Array(n * n).fill(false);
  const seq = new Array(N + 1);
  const start = cellOfClueK[1];
  used[start] = true;
  seq[1] = start;
  let found = 0;
  function bt(k) {
    if (found >= limit) return;
    if (k > N) { found++; return; }
    const must = cellOfClueK[k];
    const prev = seq[k - 1];
    for (const nb of neighbors(n, prev)) {
      if (used[nb]) continue;
      if (must !== undefined && must !== nb) continue;
      if (must === undefined && clueAtCell[nb] !== undefined && clueAtCell[nb] !== k) continue;
      used[nb] = true; seq[k] = nb;
      bt(k + 1);
      used[nb] = false;
      if (found >= limit) return;
    }
  }
  bt(2);
  return found;
}

// Build a uniquely-solvable Hidato level.
function buildPuzzle(level) {
  const n = level.n, N = n * n;
  const rng = seededRandom(level.seed);
  const path = findHamilton(n, rng);
  if (!path) return null;
  // number cells 1..N along the path
  const numAt = new Array(N);                 // cell -> number
  path.forEach((cell, i) => { numAt[cell] = i + 1; });
  // start with the two ends as clues; add more until unique
  const clueMap = { 1: path[0], [N]: path[N - 1] };
  // candidate clue positions (k between 2 and N-1), shuffled
  const candidates = [];
  for (let k = 2; k <= N - 1; k++) candidates.push(k);
  for (let i = candidates.length - 1; i > 0; i--) {
    const j = (rng() * (i + 1)) | 0;
    [candidates[i], candidates[j]] = [candidates[j], candidates[i]];
  }
  let count = solveCount(n, N, clueMap, 2);
  let added = 0;
  while (count !== 1 && added < Math.floor(N * 0.55)) {
    const k = candidates[added++];
    clueMap[k] = path[k - 1];
    count = solveCount(n, N, clueMap, 2);
  }
  if (count !== 1) return null;
  // build the puzzle: which cells are revealed
  const revealed = {};
  for (const k in clueMap) revealed[clueMap[k]] = +k;
  return { n, N, path, numAt, revealed, clueCount: Object.keys(revealed).length };
}
