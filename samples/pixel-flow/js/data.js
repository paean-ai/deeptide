// Pixel Flow - level definitions and solvable-puzzle generation.

const VW = 360, VH = 480;

const COLORS = [
  '#e8554f', '#4a9be8', '#5fc06e', '#f2cf3f', '#9a6cd8',
  '#ef9b3e', '#4fd6d6', '#ff7db0', '#a8d84a', '#c8804a',
];

// Each level is a grid size + seed; the puzzle is generated deterministically.
const LEVELS = [
  { size: 5, seed: 3121 },
  { size: 5, seed: 5417 },
  { size: 6, seed: 2298 },
  { size: 6, seed: 7741 },
  { size: 7, seed: 1903 },
  { size: 7, seed: 6056 },
  { size: 8, seed: 4482 },
  { size: 8, seed: 9135 },
  { size: 8, seed: 11260 },
  { size: 9, seed: 13744 },
  { size: 9, seed: 16018 },
  { size: 9, seed: 18395 },
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

// Carve the grid into a set of simple paths that together cover every cell.
// Each path's two ends become a colour's endpoints, so the puzzle always has
// a full-coverage solution. Returns an array of paths (arrays of {r,c}).
function genPuzzle(size, seed) {
  const rng = seededRandom(seed);
  const N = size, total = N * N;

  for (let attempt = 0; attempt < 600; attempt++) {
    const grid = new Array(total).fill(-1);
    const paths = [];
    let filled = 0;
    while (filled < total) {
      let start = -1;
      const scan = (rng() * total) | 0;
      for (let k = 0; k < total; k++) {
        const idx = (scan + k) % total;
        if (grid[idx] === -1) { start = idx; break; }
      }
      const pid = paths.length;
      const path = [start];
      grid[start] = pid;
      filled++;
      while (true) {
        const cur = path[path.length - 1];
        const r = (cur / N) | 0, c = cur % N;
        const nb = [];
        if (r > 0 && grid[cur - N] === -1) nb.push(cur - N);
        if (r < N - 1 && grid[cur + N] === -1) nb.push(cur + N);
        if (c > 0 && grid[cur - 1] === -1) nb.push(cur - 1);
        if (c < N - 1 && grid[cur + 1] === -1) nb.push(cur + 1);
        if (!nb.length) break;
        const nx = nb[(rng() * nb.length) | 0];
        path.push(nx);
        grid[nx] = pid;
        filled++;
      }
      paths.push(path);
    }
    if (paths.length >= 4 && paths.length <= COLORS.length &&
        paths.every(p => p.length >= 3)) {
      return paths.map(p => p.map(i => ({ r: (i / N) | 0, c: i % N })));
    }
  }

  // Fallback: a boustrophedon Hamiltonian path cut into equal segments.
  const order = [];
  for (let r = 0; r < N; r++) {
    if (r % 2 === 0) for (let c = 0; c < N; c++) order.push({ r, c });
    else for (let c = N - 1; c >= 0; c--) order.push({ r, c });
  }
  const segCount = Math.min(COLORS.length, Math.max(4, Math.round(total / 6)));
  const segLen = Math.ceil(total / segCount);
  const paths = [];
  for (let i = 0; i < order.length; i += segLen) paths.push(order.slice(i, i + segLen));
  return paths;
}
