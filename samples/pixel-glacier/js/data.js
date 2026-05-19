// Pixel Glacier - an ice-sliding maze. Step in a direction and you slide until
// a rock or the edge stops you. Reach the exit in as few slides as possible.
//
// Each level scatters rocks from a seed, picks a start and an exit, then a BFS
// over the slide graph confirms the exit is reachable and measures the optimal
// slide count (the par). Levels that are too short or unreachable are rejected.

const VW = 360, VH = 480;

// cell types
const ICE = 0, ROCK = 1, EXIT = 2;
// up, right, down, left
const DIRS = [[-1, 0], [0, 1], [1, 0], [0, -1]];

const LEVELS = [
  { name: ['Drift', '浮冰'],    seed: 23,  n: 6 },
  { name: ['Floe', '冰原'],     seed: 71,  n: 6 },
  { name: ['Crevasse', '冰隙'], seed: 154, n: 7 },
  { name: ['Tundra', '冻原'],   seed: 246, n: 7 },
  { name: ['Iceberg', '冰山'],  seed: 358, n: 8 },
  { name: ['Glacier', '冰川'],  seed: 489, n: 8 },
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

// Slide from `idx` in direction d until a rock or edge stops the move; landing
// on the exit also stops the slide. Returns the destination cell index.
function slideFrom(grid, n, idx, d) {
  let r = (idx / n) | 0, c = idx % n;
  const [dr, dc] = DIRS[d];
  while (true) {
    const nr = r + dr, nc = c + dc;
    if (nr < 0 || nc < 0 || nr >= n || nc >= n) break;
    const cell = grid[nr * n + nc];
    if (cell === ROCK) break;
    r = nr; c = nc;
    if (cell === EXIT) break;
  }
  return r * n + c;
}

// BFS over the slide graph: returns a map cell -> minimum slides from start.
function slideDistances(grid, n, start) {
  const dist = { [start]: 0 };
  const queue = [start];
  let head = 0;
  while (head < queue.length) {
    const cur = queue[head++];
    for (let d = 0; d < 4; d++) {
      const dest = slideFrom(grid, n, cur, d);
      if (dest === cur || dest in dist) continue;
      dist[dest] = dist[cur] + 1;
      queue.push(dest);
    }
  }
  return dist;
}

// Build a solvable ice level: scatter rocks, place start + exit, verify by BFS.
function buildPuzzle(level) {
  const n = level.n, N = n * n;
  const rng = seededRandom(level.seed);
  const minPar = 5 + (n - 6);

  for (let attempt = 0; attempt < 4000; attempt++) {
    const grid = new Array(N).fill(ICE);
    for (let i = 0; i < N; i++) if (rng() < 0.2) grid[i] = ROCK;
    const iceCells = [];
    for (let i = 0; i < N; i++) if (grid[i] === ICE) iceCells.push(i);
    if (iceCells.length < N * 0.55) continue;
    const start = iceCells[(rng() * iceCells.length) | 0];
    let exit = iceCells[(rng() * iceCells.length) | 0];
    if (exit === start) continue;
    grid[exit] = EXIT;
    const dist = slideDistances(grid, n, start);
    if (!(exit in dist)) continue;
    const par = dist[exit];
    if (par < minPar || par > 40) continue;
    return { n, grid, start, exit, par };
  }
  return null;
}
